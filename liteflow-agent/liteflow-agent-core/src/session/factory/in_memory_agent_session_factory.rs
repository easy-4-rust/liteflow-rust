use std::sync::Arc;

use agentscope_core::session::{InMemorySession, Session};

use super::AgentSessionFactory;
use crate::{AgentConfig, AgentConfigException, MemoryStorageMode};

/// 使用 AgentScope 进程内存储支持 `JVM` 模式。
///
/// 状态可在同一进程内跨调用保留，进程退出后丢失。
///
/// 对应 Java: `com.yomahub.liteflow.agent.session.factory.InMemoryAgentSessionFactory`。
pub struct InMemoryAgentSessionFactory;

impl AgentSessionFactory for InMemoryAgentSessionFactory {
    fn mode(&self) -> MemoryStorageMode {
        MemoryStorageMode::Jvm
    }

    fn create(
        &self,
        _agent_config: &AgentConfig,
    ) -> Result<Option<Arc<dyn Session>>, AgentConfigException> {
        Ok(Some(Arc::new(InMemorySession::new())))
    }
}
