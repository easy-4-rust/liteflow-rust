//! 对应 Java 类：com.yomahub.liteflow.spi.CmpAroundAspect
//!
//! 组件全局切面 SPI 接口。本 trait 按 Java 原样定义：
//! sync、`beforeProcess(nodeId, slot)` / `afterProcess(nodeId, slot)`。
//!
//! 注意：`crate::aop::ICmpAroundAspect` 是面向业务组件注册的异步拦截器；
//! 是当前 node.rs 执行路径的运行时挂接形态；本 trait 对齐 Java SPI 定义，
//! 供 SPI 体系（holder/local）按 Java 语义装载。

use crate::slot::Slot;

use super::spi_priority::SpiPriority;

/// 对应 CmpAroundAspect
pub trait CmpAroundAspect: SpiPriority + Send + Sync {
    /// 对应 beforeProcess(String nodeId, Slot slot)
    fn before_process(&self, node_id: &str, slot: &Slot);

    /// 对应 afterProcess(String nodeId, Slot slot)
    fn after_process(&self, node_id: &str, slot: &Slot);
}
