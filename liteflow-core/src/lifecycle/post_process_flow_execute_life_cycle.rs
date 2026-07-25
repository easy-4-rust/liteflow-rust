//! 对应 com.yomahub.liteflow.lifecycle.PostProcessFlowExecuteLifeCycle（2.11+）：
//! 流程执行前后的生命周期钩子（postProcessBeforeFlowExecute / postProcessAfterFlowExecute）。

use async_trait::async_trait;

/// 流程执行生命周期钩子
#[async_trait]
pub trait PostProcessFlowExecuteLifeCycle: Send + Sync + 'static {
    /// postProcessBeforeFlowExecute(chainId)
    async fn post_process_before_flow_execute(&self, _chain_id: &str) {}
    /// postProcessAfterFlowExecute(chainId)
    async fn post_process_after_flow_execute(&self, _chain_id: &str) {}
}
