use std::sync::Arc;

use agentscope_core::session::Session;

use super::AgentSessionFactory;
use crate::{AgentConfig, AgentConfigException, MemoryStorageMode};

/// 返回空 Session，通知 Agent 运行时跳过所有记忆加载和保存。
///
/// 对应 Java: `com.yomahub.liteflow.agent.session.factory.NoneAgentSessionFactory`。
pub struct NoneAgentSessionFactory;

impl AgentSessionFactory for NoneAgentSessionFactory {
    fn mode(&self) -> MemoryStorageMode {
        MemoryStorageMode::None
    }

    fn create(
        &self,
        _agent_config: &AgentConfig,
    ) -> Result<Option<Arc<dyn Session>>, AgentConfigException> {
        Ok(None)
    }
}
