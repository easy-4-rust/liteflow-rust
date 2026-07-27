use std::sync::Arc;

use super::AgentSessionFactory;

/// 供外部 crate 通过 `inventory::submit!` 注册 Session 工厂构造函数。
///
/// 对应 Java: 无（Rust 对 Java ServiceLoader SPI 的 `inventory` 映射）。
pub struct AgentSessionFactoryRegistration {
    /// 创建一个新的线程安全工厂实例。
    pub factory: fn() -> Arc<dyn AgentSessionFactory>,
}

inventory::collect!(AgentSessionFactoryRegistration);
