use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use agentscope_core::session::Session;

use super::{
    AgentSessionFactory, AgentSessionFactoryRegistration, InMemoryAgentSessionFactory,
    LocalFileAgentSessionFactory, MysqlAgentSessionFactory, NoneAgentSessionFactory,
    RedisAgentSessionFactory,
};
use crate::{AgentConfig, AgentConfigException, MemoryStorageMode};

/// 按记忆模式解析 Agent Session 工厂的注册表。
///
/// 注册顺序为内置工厂优先装入、`inventory` 外部工厂随后覆盖，因此应用可以替换
/// 本地文件加密策略或接入新的后端实现。
///
/// 对应 Java: `com.yomahub.liteflow.agent.session.factory.AgentSessionFactoryRegistry`。
pub struct AgentSessionFactoryRegistry {
    factories: RwLock<HashMap<MemoryStorageMode, Arc<dyn AgentSessionFactory>>>,
}

impl AgentSessionFactoryRegistry {
    /// 创建包含全部内置工厂和 inventory 外部覆盖项的注册表。
    #[must_use]
    pub fn new() -> Self {
        let registry = Self {
            factories: RwLock::new(HashMap::new()),
        };
        registry.register(Arc::new(InMemoryAgentSessionFactory));
        registry.register(Arc::new(LocalFileAgentSessionFactory));
        registry.register(Arc::new(RedisAgentSessionFactory));
        registry.register(Arc::new(MysqlAgentSessionFactory));
        registry.register(Arc::new(NoneAgentSessionFactory));
        for registration in inventory::iter::<AgentSessionFactoryRegistration> {
            registry.register((registration.factory)());
        }
        registry
    }

    /// 注册或覆盖一个模式对应的 Session 工厂。
    ///
    /// 对应 Java: `AgentSessionFactoryRegistry#register`。
    pub fn register(&self, factory: Arc<dyn AgentSessionFactory>) {
        self.factories
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(factory.mode(), factory);
    }

    /// 根据配置中的记忆模式构造 Session。
    ///
    /// `NONE` 返回 `Ok(None)`；没有对应工厂或工厂构造失败时返回配置错误。
    ///
    /// 对应 Java: `AgentSessionFactoryRegistry#createSession`。
    pub fn create_session(
        &self,
        config: &AgentConfig,
    ) -> Result<Option<Arc<dyn Session>>, AgentConfigException> {
        let mode = config.session.memory.mode;
        let factory = self
            .factories
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&mode)
            .cloned()
            .ok_or_else(|| {
                AgentConfigException::new(format!(
                    "No AgentSessionFactory registered for mode: {mode:?}"
                ))
            })?;
        factory.create(config)
    }
}

impl Default for AgentSessionFactoryRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use agentscope_core::session::Session;

    use super::AgentSessionFactoryRegistry;
    use crate::{AgentConfig, AgentConfigException, AgentSessionFactory, MemoryStorageMode};

    struct StatelessJvmOverride;

    impl AgentSessionFactory for StatelessJvmOverride {
        fn mode(&self) -> MemoryStorageMode {
            MemoryStorageMode::Jvm
        }

        fn create(
            &self,
            _agent_config: &AgentConfig,
        ) -> Result<Option<Arc<dyn Session>>, AgentConfigException> {
            Ok(None)
        }
    }

    #[test]
    fn builtins_cover_all_modes_and_explicit_registration_overrides() {
        let registry = AgentSessionFactoryRegistry::new();
        let config = AgentConfig::default();
        assert!(
            registry
                .create_session(&config)
                .expect("JVM 工厂应可用")
                .is_some()
        );

        registry.register(Arc::new(StatelessJvmOverride));
        assert!(
            registry
                .create_session(&config)
                .expect("覆盖工厂应可用")
                .is_none()
        );
    }
}
