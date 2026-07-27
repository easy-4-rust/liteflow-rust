//! LiteFlow Agent 桥接层错误对象。

use thiserror::Error;

use crate::MemoryStorageMode;

/// Agent 配置、提示词或执行失败。
#[derive(Debug, Error)]
pub enum AgentError {
    /// 由独立 `AgentConfigException` 收敛而来的配置错误。
    #[error("{0}")]
    Config(String),
    /// Agent key 为空。
    #[error("agent key cannot be blank")]
    BlankAgentKey,
    /// 用户提示词为空。
    #[error("agent user prompt cannot be blank")]
    BlankUserPrompt,
    /// 本地文件记忆模式缺少工作区根目录。
    #[error("liteflow.agent.workspace.root is required for LOCAL_FILE memory")]
    WorkspaceRootRequired,
    /// 工作区根目录不存在且配置禁止自动创建。
    #[error("workspace root does not exist: {0}")]
    WorkspaceRootDoesNotExist(String),
    /// 工作区路径越界或不满足相对路径约束。
    #[error("workspace path denied: {0}")]
    WorkspacePathDenied(String),
    /// 工作区文件操作失败。
    #[error("workspace {operation} failed: {message}")]
    WorkspaceIo {
        /// 操作名称。
        operation: &'static str,
        /// 底层错误信息。
        message: String,
    },
    /// Skills 目录、技能声明或技能文件加载失败。
    #[error("{0}")]
    SkillsLoad(String),
    /// Redis/MySQL 客户端必须由宿主构造并显式注入 AgentScope Session。
    #[error("memory backend {0:?} requires an explicitly injected AgentScope Session")]
    SessionBackendRequiresInjection(MemoryStorageMode),
    /// AgentScope 执行失败。
    #[error("AgentScope execution failed: {0}")]
    Execution(String),
}

impl From<super::AgentConfigException> for AgentError {
    fn from(error: super::AgentConfigException) -> Self {
        Self::Config(error.to_string())
    }
}

impl From<super::AgentInvocationException> for AgentError {
    fn from(error: super::AgentInvocationException) -> Self {
        Self::Execution(error.to_string())
    }
}
