//! 可回滚元素接口。

use crate::exception::LFResult;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;

/// 流程执行失败后可进行补偿的元素。
///
/// Java 当前只有 `Node` 实现本接口；Rust 保留相同边界，由 Node 把 Slot/RefNode
/// 转换为 `CmpContext` 后调用组件的 `rollback`。
///
/// 对应 Java: `com.yomahub.liteflow.flow.element.Rollbackable`。
#[async_trait]
pub trait Rollbackable: Send + Sync {
    /// 回滚当前元素。
    ///
    /// 参数 `ctx` 对应 Java 的 `slotIndex`，`frame` 保留循环与 bind 执行路径。
    /// 对应 Java: `Rollbackable#rollback`。
    async fn rollback(&self, ctx: &Ctx, frame: &Frame) -> LFResult<()>;
}
