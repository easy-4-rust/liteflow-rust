//! 对应 com.yomahub.liteflow.lifecycle.PostProcessFlowExecuteLifeCycle（2.11+）：
//! 流程执行前后的生命周期钩子（postProcessBeforeFlowExecute / postProcessAfterFlowExecute）。

use super::life_cycle::LifeCycle;
use async_trait::async_trait;

/// 流程执行生命周期钩子
#[async_trait]
pub trait PostProcessFlowExecuteLifeCycle: LifeCycle {
    /// postProcessBeforeFlowExecute(chainId)
    async fn post_process_before_flow_execute(&self, chain_id: &str) {
        // 默认生命周期不改变执行状态；显式消费参数，表明这是有意的 no-op。
        let _ = chain_id;
    }
    /// postProcessAfterFlowExecute(chainId)
    async fn post_process_after_flow_execute(&self, chain_id: &str) {
        // 默认生命周期不改变执行状态；实现方可按需覆盖。
        let _ = chain_id;
    }
}
