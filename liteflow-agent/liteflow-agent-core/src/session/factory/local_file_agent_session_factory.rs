use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use agentscope_core::session::{JsonSession, Session};

use super::AgentSessionFactory;
use crate::{AgentConfig, AgentConfigException, LocalFileMemoryConfig, MemoryStorageMode};

/// 把 JSON Session 存储在 `workspace.root/.agent-session/`。
///
/// 持久化目录与 conversation 工作区平级，工作区过期清理不会误删会话记忆。
///
/// 对应 Java: `com.yomahub.liteflow.agent.session.factory.LocalFileAgentSessionFactory`。
pub struct LocalFileAgentSessionFactory;

impl AgentSessionFactory for LocalFileAgentSessionFactory {
    fn mode(&self) -> MemoryStorageMode {
        MemoryStorageMode::LocalFile
    }

    fn create(
        &self,
        agent_config: &AgentConfig,
    ) -> Result<Option<Arc<dyn Session>>, AgentConfigException> {
        let workspace_root = agent_config.workspace.root.as_deref().ok_or_else(|| {
            AgentConfigException::new(
                "liteflow.agent.workspace.root is required when \
                 session.memory.mode=LOCAL_FILE",
            )
        })?;
        let storage_directory = PathBuf::from(workspace_root).join(LocalFileMemoryConfig::SUB_DIR);

        // 先显式创建目录，确保配置错误在首次构造 Session 时同步暴露。
        fs::create_dir_all(&storage_directory).map_err(|error| {
            AgentConfigException::with_source(
                format!(
                    "cannot create session storage dir: {}",
                    storage_directory.display()
                ),
                error,
            )
        })?;
        Ok(Some(Arc::new(JsonSession::with_dir(storage_directory))))
    }
}
