//! 对应 Java 类：com.yomahub.liteflow.spi.CmpAroundAspect
//!
//! 组件全局切面 SPI 接口。本 trait 按 Java 原样定义：
//! `beforeProcess(NodeComponent)` / `afterProcess(NodeComponent)`。
//!
//! 注意：`crate::aop::ICmpAroundAspect` 是面向业务组件注册的异步拦截器；
//! 是当前 node.rs 执行路径的运行时挂接形态；本 trait 对齐 Java SPI 定义，
//! 供 SPI 体系（holder/local）按 Java 语义装载。

use crate::exception::LiteflowError;
use crate::slot::CmpContext;

use super::spi_priority::SpiPriority;

/// 对应 CmpAroundAspect
pub trait CmpAroundAspect: SpiPriority + Send + Sync {
    /// 在组件执行前调用全局切面。
    ///
    /// 参数 `context` 携带 Java `NodeComponent` 在本次执行中的节点、Slot 与
    /// Frame 信息。对应 Java: `CmpAroundAspect#beforeProcess(NodeComponent)`。
    fn before_process(&self, context: &CmpContext);

    /// 在组件 finally 阶段调用全局切面。
    ///
    /// 参数 `context` 为当前组件执行上下文。对应 Java:
    /// `CmpAroundAspect#afterProcess(NodeComponent)`。
    fn after_process(&self, context: &CmpContext);

    /// 对应 onSuccess(NodeComponent cmp)。
    ///
    /// Rust 以不可变执行上下文替代 Java 可变组件引用，避免跨线程共享组件内部
    /// 状态。参数 `context` 标识当前节点和执行槽。
    fn on_success(&self, context: &CmpContext);

    /// 对应 onError(NodeComponent cmp, Exception e)。
    ///
    /// 参数 `context`、`error` 分别标识当前执行上下文和原始执行错误。
    fn on_error(&self, context: &CmpContext, error: &LiteflowError);
}
