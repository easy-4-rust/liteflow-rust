//! LiteFlow 生命周期扩展点的统一父接口。

use std::sync::Arc;

use super::life_cycle_holder::LifeCycleHolder;

/// 生命周期扩展点的父接口。
///
/// 对应 Java: `com.yomahub.liteflow.lifecycle.LifeCycle`。
/// Java 接口不声明方法，只用于把不同阶段的生命周期实现归为同一类对象。
/// Rust 还需要在没有 JVM `isAssignableFrom` 反射的前提下完成动态分类，因此
/// 由实现对象把自身注册到 `LifeCycleHolder` 的对应强类型列表中。这个入口只
/// 承担 Java 反射分派的 Rust 化映射，具体回调仍由各阶段子 trait 定义。
///
/// 对应 Java: `com.yomahub.liteflow.lifecycle.LifeCycle`。
pub trait LifeCycle: Send + Sync + 'static {
    /// 把当前生命周期对象登记到持有器的对应阶段列表。
    ///
    /// - `self`: 共享生命周期对象；
    /// - `life_cycle_holder`: 当前 `FlowBus` 隔离持有的生命周期注册表。
    ///
    /// 对应 Java: `LifeCycleHolder#addLifeCycle` 中基于接口类型的分类逻辑。
    fn register_life_cycle(self: Arc<Self>, life_cycle_holder: &mut LifeCycleHolder);
}
