use std::sync::Arc;

use agentscope_core::session::Session;

use super::AgentSessionFactory;
use crate::{AgentConfig, AgentConfigException, MemoryStorageMode};

/// Redis Session 工厂边界。
///
/// Java 可从 Spring Bean 反射适配 Redisson/Jedis/Lettuce；Rust 无 JVM Bean 类型擦除，
/// 因此宿主必须通过 `ReActAgentComponentBuilder::session` 显式注入已构造的
/// AgentScope RedisSession，避免伪造不可用客户端。
///
/// 对应 Java: `com.yomahub.liteflow.agent.session.factory.RedisAgentSessionFactory`。
pub struct RedisAgentSessionFactory;

impl AgentSessionFactory for RedisAgentSessionFactory {
    fn mode(&self) -> MemoryStorageMode {
        MemoryStorageMode::Redis
    }

    fn create(
        &self,
        _agent_config: &AgentConfig,
    ) -> Result<Option<Arc<dyn Session>>, AgentConfigException> {
        Err(AgentConfigException::new(
            "memory backend Redis requires an explicitly injected AgentScope Session",
        ))
    }
}
