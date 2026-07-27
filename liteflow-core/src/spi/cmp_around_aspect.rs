//! 对应 Java 类：com.yomahub.liteflow.spi.CmpAroundAspect
//!
//! 组件全局切面 SPI 接口。本 trait 按 Java 原样定义：
//! sync、`beforeProcess(nodeId, slot)` / `afterProcess(nodeId, slot)`。
//!
//! 注意：`crate::aop::ICmpAroundAspect` 是面向业务组件注册的异步拦截器；
//! 是当前 node.rs 执行路径的运行时挂接形态；本 trait 对齐 Java SPI 定义，
//! 供 SPI 体系（holder/local）按 Java 语义装载。

use crate::exception::LiteflowError;
use crate::slot::Slot;

use super::spi_priority::SpiPriority;

/// 对应 CmpAroundAspect
pub trait CmpAroundAspect: SpiPriority + Send + Sync {
    /// 对应 beforeProcess(String nodeId, Slot slot)
    fn before_process(&self, node_id: &str, slot: &Slot);

    /// 对应 afterProcess(String nodeId, Slot slot)
    fn after_process(&self, node_id: &str, slot: &Slot);

    /// 对应 onSuccess(NodeComponent cmp)。
    ///
    /// Rust 以节点 ID 与当前 Slot 替代 Java 可变组件引用，避免跨线程共享组件内部
    /// 状态。参数 `node_id`、`slot` 分别标识当前节点和执行槽。
    fn on_success(&self, node_id: &str, slot: &Slot);

    /// 对应 onError(NodeComponent cmp, Exception e)。
    ///
    /// 参数 `node_id`、`slot`、`error` 分别标识当前节点、执行槽和原始执行错误。
    fn on_error(&self, node_id: &str, slot: &Slot, error: &LiteflowError);
}
