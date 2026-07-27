//! Script Hash 轮询任务。

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use liteflow_core::exception::LFResult;
use liteflow_core::parser::helper::NodeConvertHelper;
use liteflow_core::rule_plugin::RuleSourceWatcher;

use crate::parser::redis::mode::RClient;

/// 检测 Redis Script Hash 字段新增、修改和删除。
///
/// 对应 Java:
/// `com.yomahub.liteflow.parser.redis.mode.polling.ScriptPollingTask`。
#[derive(Clone)]
pub struct ScriptPollingTask {
    script_client: RClient,
    script_key: String,
    last_fingerprint: Arc<Mutex<String>>,
    last_node_ids: Arc<Mutex<HashSet<String>>>,
}

impl ScriptPollingTask {
    /// 读取当前 Script Hash 指纹并创建任务。
    pub fn new(script_client: RClient, script_key: impl Into<String>) -> LFResult<Self> {
        let script_key = script_key.into();
        let scripts = script_client.get_map(&script_key)?;
        let fingerprint = fingerprint(&scripts);
        let node_ids = enabled_node_ids(&scripts);
        Ok(Self {
            script_client,
            script_key,
            last_fingerprint: Arc::new(Mutex::new(fingerprint)),
            last_node_ids: Arc::new(Mutex::new(node_ids)),
        })
    }

    /// 执行一次 Script Hash 检测；发生变化时触发规则对账。
    ///
    /// 返回值表示本轮是否检测到变化。对应 Java `ScriptPollingTask#run`。
    pub async fn run(&self, watcher: &RuleSourceWatcher) -> LFResult<bool> {
        let scripts = self.script_client.get_map(&self.script_key)?;
        let fingerprint = fingerprint(&scripts);
        {
            let previous = self
                .last_fingerprint
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if *previous == fingerprint {
                return Ok(false);
            }
        }

        let current_node_ids = enabled_node_ids(&scripts);
        watcher.reload().await?;
        let mut previous_node_ids = self
            .last_node_ids
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for removed_node_id in previous_node_ids.difference(&current_node_ids) {
            watcher.unload_script_node(removed_node_id);
        }
        *previous_node_ids = current_node_ids;
        *self
            .last_fingerprint
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = fingerprint;
        Ok(true)
    }
}

fn enabled_node_ids(scripts: &HashMap<String, String>) -> HashSet<String> {
    scripts
        .keys()
        .filter_map(|field| NodeConvertHelper::convert(field))
        .filter(|node| node.enable())
        .map(|node| node.node_id().to_string())
        .collect()
}

fn fingerprint(scripts: &HashMap<String, String>) -> String {
    let mut entries: Vec<_> = scripts.iter().collect();
    entries.sort_by(|left, right| left.0.cmp(right.0));
    let content = entries
        .into_iter()
        .map(|(field, value)| format!("{field}\0{value}\0"))
        .collect::<String>();
    liteflow_core::rule_plugin::fnv_fp(&content)
}
