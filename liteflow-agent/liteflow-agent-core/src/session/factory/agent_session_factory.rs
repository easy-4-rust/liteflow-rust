use std::sync::Arc;

use agentscope_core::session::Session;

use crate::{AgentConfig, AgentConfigException, MemoryStorageMode};

/// 为 AgentScope Session 接入额外持久化后端的工厂 SPI。
///
/// Rust 使用 `inventory` 注册外部工厂；同一模式出现多个工厂时，registry 后加载的
/// 外部工厂覆盖内置实现。
///
/// 对应 Java: `com.yomahub.liteflow.agent.session.factory.AgentSessionFactory`。
pub trait AgentSessionFactory: Send + Sync {
    /// 返回当前工厂处理的唯一记忆模式。
    ///
    /// 对应 Java: `AgentSessionFactory#mode`。
    fn mode(&self) -> MemoryStorageMode;

    /// 根据 Agent 配置构造底层 Session。
    ///
    /// `MemoryStorageMode::None` 返回 `Ok(None)`；其他可用后端返回共享 Session。
    ///
    /// 对应 Java: `AgentSessionFactory#create`。
    fn create(
        &self,
        agent_config: &AgentConfig,
    ) -> Result<Option<Arc<dyn Session>>, AgentConfigException>;
}

/// 供外部 crate 通过 `inventory::submit!` 注册 Session 工厂构造函数。
pub struct AgentSessionFactoryRegistration {
    /// 创建一个新的线程安全工厂实例。
    pub factory: fn() -> Arc<dyn AgentSessionFactory>,
}

inventory::collect!(AgentSessionFactoryRegistration);
