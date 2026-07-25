//! 对应 lifecycle 包：构建/执行生命周期钩子 SPI。

use async_trait::async_trait;

/// 对应 PostProcessNodeBuildLifeCycle
pub trait PostProcessNodeBuildLifeCycle: Send + Sync + 'static {
    fn post_process_after_node_build(&self, node_id: &str);
}

/// 对应 PostProcessChainBuildLifeCycle
pub trait PostProcessChainBuildLifeCycle: Send + Sync + 'static {
    fn post_process_after_chain_build(&self, chain_id: &str);
}

/// 对应 PostProcessFlowExecuteLifeCycle
#[async_trait]
pub trait PostProcessFlowExecuteLifeCycle: Send + Sync + 'static {
    async fn post_process_before_flow_execute(&self, _chain_id: &str) {}
    async fn post_process_after_flow_execute(&self, _chain_id: &str) {}
}

/// 对应 PostProcessChainExecuteLifeCycle
#[async_trait]
pub trait PostProcessChainExecuteLifeCycle: Send + Sync + 'static {
    async fn post_process_before_chain_execute(&self, _chain_id: &str) {}
    async fn post_process_after_chain_execute(&self, _chain_id: &str) {}
}
