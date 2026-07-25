//! 对应 Java 类：com.yomahub.liteflow.spi.SpiPriority
//!
//! SPI 实现的优先级接口：数字越小优先级越高。
//! Java 侧 Holder 通过 ServiceLoader 加载全部实现后按 priority 升序取首个；
//! Rust 侧以显式 register 替代 ServiceLoader，priority 保留用于多实现排序语义。

/// 对应 SpiPriority
pub trait SpiPriority {
    /// 对应 priority()：数字越小优先级越高
    fn priority(&self) -> i32;
}
