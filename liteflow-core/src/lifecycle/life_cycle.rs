//! LiteFlow 生命周期扩展点的统一标记接口。

/// 生命周期扩展点的父接口。
///
/// 对应 Java: `com.yomahub.liteflow.lifecycle.LifeCycle`。
/// Java 接口不声明方法，只用于把不同阶段的生命周期实现归为同一类对象。
/// Rust 通过 blanket implementation 保留相同的标记语义，具体回调仍由各阶段
/// 的子 trait 定义。
pub trait LifeCycle: Send + Sync + 'static {}

impl<T> LifeCycle for T where T: Send + Sync + 'static + ?Sized {}
