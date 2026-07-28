//! Chain 执行前后的生命周期钩子。
//!
//! 对应 Java:
//! `com.yomahub.liteflow.lifecycle.PostProcessChainExecuteLifeCycle`。

use super::life_cycle::LifeCycle;
use crate::slot::Slot;
use async_trait::async_trait;

/// 在每个主体 Chain 执行前后接收 Chain ID 与本次共享数据槽。
///
/// 子 Chain 与主 Chain 都会触发本接口；决策路由 `executeRoute` 不触发，与 Java
/// `Chain#execute` 和 `Chain#executeRoute` 的边界一致。
/// 对应 Java:
/// `com.yomahub.liteflow.lifecycle.PostProcessChainExecuteLifeCycle`。
#[async_trait]
pub trait PostProcessChainExecuteLifeCycle: LifeCycle {
    /// 在指定 Chain 的主体 Condition 开始执行前调用。
    ///
    /// 参数 `chain_id` 是当前主链或子链 ID；`slot` 是所有嵌套执行共享的数据槽。
    /// 对应 Java:
    /// `PostProcessChainExecuteLifeCycle#postProcessBeforeChainExecute`。
    async fn post_process_before_chain_execute(&self, chain_id: &str, slot: &Slot);

    /// 在指定 Chain 的主体 Condition 执行结束后调用。
    ///
    /// 成功、组件异常与主动结束都会进入该回调。参数 `slot` 可读取执行结果和异常。
    /// 对应 Java:
    /// `PostProcessChainExecuteLifeCycle#postProcessAfterChainExecute`。
    async fn post_process_after_chain_execute(&self, chain_id: &str, slot: &Slot);
}
