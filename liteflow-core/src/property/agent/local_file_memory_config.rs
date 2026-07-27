use serde::{Deserialize, Serialize};

/// 本地文件记忆后端配置。
///
/// 记忆固定存储在 `workspace.root/.agent-session/`，与每个会话的工作目录平级，
/// 避免工作区清理误删持久化记忆。自定义位置应由调用方注入 AgentScope Session。
///
/// 对应 Java: `com.yomahub.liteflow.property.agent.LocalFileMemoryConfig`。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalFileMemoryConfig;

impl LocalFileMemoryConfig {
    /// 会话 JSON 文件位于工作区根目录下的固定子目录名。
    ///
    /// 对应 Java: `LocalFileMemoryConfig#SUB_DIR`。
    pub const SUB_DIR: &'static str = ".agent-session";
}
