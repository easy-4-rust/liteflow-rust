//! 对应 liteflow-rule-zk：基于 zookeeper crate（bonifaido/rust-zookeeper）。

use super::rule_source::{fnv_fp, RuleFormat, RuleSource};
use crate::exception::{LFResult, LiteflowError};
use async_trait::async_trait;

/// Zookeeper 规则源（对应 ZkParser：读取 znode 数据）
pub struct ZkRuleSource {
    /// connect string，如 "127.0.0.1:2181"
    pub connect_string: String,
    /// 规则节点路径（LiteFlow 默认 /lite-flow/flow）
    pub node_path: String,
    pub format: RuleFormat,
}

pub struct NopWatcher;
impl zookeeper::Watcher for NopWatcher {
    fn handle(&self, _e: zookeeper::WatchedEvent) {}
}

#[async_trait]
impl RuleSource for ZkRuleSource {
    async fn fetch(&self) -> LFResult<(String, String)> {
        let connect = self.connect_string.clone();
        let path = self.node_path.clone();
        let text = tokio::task::spawn_blocking(move || -> LFResult<String> {
            let zk = zookeeper::ZooKeeper::connect(
                &connect,
                std::time::Duration::from_secs(5),
                NopWatcher,
            )
            .map_err(|e| LiteflowError::Rule(format!("zk connect error: {e}")))?;
            let (data, _stat) = zk
                .get_data(&path, false)
                .map_err(|e| LiteflowError::Rule(format!("zk get data error: {e}")))?;
            zk.close().ok();
            String::from_utf8(data).map_err(|e| LiteflowError::Rule(format!("zk decode error: {e}")))
        })
        .await
        .map_err(|e| LiteflowError::Rule(format!("zk task error: {e}")))??;
        let fp = fnv_fp(&text);
        Ok((text, fp))
    }
    fn format(&self) -> RuleFormat {
        self.format
    }
    fn name(&self) -> &str {
        "zookeeper"
    }
}
