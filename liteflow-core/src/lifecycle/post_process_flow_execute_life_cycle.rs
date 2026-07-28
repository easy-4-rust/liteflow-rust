//! FlowExecutor 执行前后的生命周期钩子。
//!
//! 对应 Java:
//! `com.yomahub.liteflow.lifecycle.PostProcessFlowExecuteLifeCycle`。

use super::life_cycle::LifeCycle;
use crate::slot::Slot;
use async_trait::async_trait;

/// 在一次完整 FlowExecutor 调用前后接收 Chain ID 与共享数据槽。
///
/// 该生命周期包围 Chain 查找、执行和异常处理，因此 Chain 不存在时仍会触发
/// after。对应 Java:
/// `com.yomahub.liteflow.lifecycle.PostProcessFlowExecuteLifeCycle`。
#[async_trait]
pub trait PostProcessFlowExecuteLifeCycle: LifeCycle {
    /// 在 FlowExecutor 开始处理指定 Chain 前调用。
    ///
    /// 参数 `chain_id` 是调用方请求的 Chain ID；`slot` 是本次执行的数据槽。
    /// 对应 Java:
    /// `PostProcessFlowExecuteLifeCycle#postProcessBeforeFlowExecute`。
    async fn post_process_before_flow_execute(&self, chain_id: &str, slot: &Slot);

    /// 在 FlowExecutor 完成清理前调用。
    ///
    /// 参数 `slot` 保留执行步骤、响应和异常，供监控或业务生命周期实现读取。
    /// 对应 Java:
    /// `PostProcessFlowExecuteLifeCycle#postProcessAfterFlowExecute`。
    async fn post_process_after_flow_execute(&self, chain_id: &str, slot: &Slot);
}
