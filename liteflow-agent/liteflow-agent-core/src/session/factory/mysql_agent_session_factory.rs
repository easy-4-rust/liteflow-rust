use std::sync::Arc;

use agentscope_core::session::Session;

use super::AgentSessionFactory;
use crate::{AgentConfig, AgentConfigException, MemoryStorageMode};

/// MySQL Session 工厂边界。
///
/// Java 从 Spring `DataSource` Bean 构造 Session；Rust 宿主必须通过
/// `ReActAgentComponentBuilder::session` 注入已连接的 AgentScope MysqlSession，
/// 使连接池所有权与密钥管理保持在应用边界。
///
/// 对应 Java: `com.yomahub.liteflow.agent.session.factory.MysqlAgentSessionFactory`。
pub struct MysqlAgentSessionFactory;

impl AgentSessionFactory for MysqlAgentSessionFactory {
    fn mode(&self) -> MemoryStorageMode {
        MemoryStorageMode::Mysql
    }

    fn create(
        &self,
        _agent_config: &AgentConfig,
    ) -> Result<Option<Arc<dyn Session>>, AgentConfigException> {
        Err(AgentConfigException::new(
            "memory backend Mysql requires an explicitly injected AgentScope Session",
        ))
    }
}
