//! 对应 com.yomahub.liteflow.lifecycle.PostProcessChainExecuteLifeCycle（2.11+）：
//! 链路执行前后的生命周期钩子（postProcessBeforeChainExecute / postProcessAfterChainExecute）。

use super::life_cycle::LifeCycle;
use async_trait::async_trait;

/// 链路执行生命周期钩子
#[async_trait]
pub trait PostProcessChainExecuteLifeCycle: LifeCycle {
    /// postProcessBeforeChainExecute(chainId)
    async fn post_process_before_chain_execute(&self, chain_id: &str) {
        // 默认生命周期不改变执行状态；显式消费参数，表明这是有意的 no-op。
        let _ = chain_id;
    }
    /// postProcessAfterChainExecute(chainId)
    async fn post_process_after_chain_execute(&self, chain_id: &str) {
        // 默认生命周期不改变执行状态；实现方可按需覆盖。
        let _ = chain_id;
    }
}
