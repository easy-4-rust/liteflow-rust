//! Chain Hash 轮询任务。

use std::sync::{Arc, Mutex};

use liteflow_core::exception::LFResult;
use liteflow_core::rule_plugin::RuleSourceWatcher;

use crate::parser::redis::mode::RClient;

/// 检测 Redis Chain Hash 字段新增、修改和删除。
///
/// 对应 Java:
/// `com.yomahub.liteflow.parser.redis.mode.polling.ChainPollingTask`。
#[derive(Clone)]
pub struct ChainPollingTask {
    chain_client: RClient,
    chain_key: String,
    last_fingerprint: Arc<Mutex<String>>,
}

impl ChainPollingTask {
    /// 读取当前 Chain Hash 指纹并创建任务。
    ///
    /// 参数对应 Java 构造器中的 `chainClient`、`redisParserVO.chainKey` 和
    /// `chainSHAMap` 初始状态。
    pub fn new(chain_client: RClient, chain_key: impl Into<String>) -> LFResult<Self> {
        let chain_key = chain_key.into();
        let fingerprint = chain_client.hash_fingerprint(&chain_key)?;
        Ok(Self {
            chain_client,
            chain_key,
            last_fingerprint: Arc::new(Mutex::new(fingerprint)),
        })
    }

    /// 执行一次 Chain Hash 检测；发生变化时触发规则对账。
    ///
    /// 返回值表示本轮是否检测到变化。对应 Java `ChainPollingTask#run`。
    pub async fn run(&self, watcher: &RuleSourceWatcher) -> LFResult<bool> {
        let fingerprint = self.chain_client.hash_fingerprint(&self.chain_key)?;
        {
            let previous = self
                .last_fingerprint
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if *previous == fingerprint {
                return Ok(false);
            }
        }

        // 只有完整规则成功解析并对账后，才提交新指纹；失败会在下轮继续重试。
        watcher.reload().await?;
        *self
            .last_fingerprint
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = fingerprint;
        Ok(true)
    }
}
