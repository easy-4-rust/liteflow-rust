//! 对应 com.yomahub.liteflow.lifecycle.PostProcessChainExecuteLifeCycle（2.11+）：
//! 链路执行前后的生命周期钩子（postProcessBeforeChainExecute / postProcessAfterChainExecute）。

use async_trait::async_trait;

/// 链路执行生命周期钩子
#[async_trait]
pub trait PostProcessChainExecuteLifeCycle: Send + Sync + 'static {
    /// postProcessBeforeChainExecute(chainId)
    async fn post_process_before_chain_execute(&self, _chain_id: &str) {}
    /// postProcessAfterChainExecute(chainId)
    async fn post_process_after_chain_execute(&self, _chain_id: &str) {}
}
