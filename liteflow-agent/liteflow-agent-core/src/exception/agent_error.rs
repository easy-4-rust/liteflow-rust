//! LiteFlow Agent 桥接层错误对象。

use thiserror::Error;

use crate::MemoryStorageMode;

/// Agent 配置、提示词或执行失败。
#[derive(Debug, Error)]
pub enum AgentError {
    /// Agent key 为空。
    #[error("agent key cannot be blank")]
    BlankAgentKey,
    /// 用户提示词为空。
    #[error("agent user prompt cannot be blank")]
    BlankUserPrompt,
    /// 本地文件记忆模式缺少工作区根目录。
    #[error("liteflow.agent.workspace.root is required for LOCAL_FILE memory")]
    WorkspaceRootRequired,
    /// Redis/MySQL 客户端必须由宿主构造并显式注入 AgentScope Session。
    #[error("memory backend {0:?} requires an explicitly injected AgentScope Session")]
    SessionBackendRequiresInjection(MemoryStorageMode),
    /// AgentScope 执行失败。
    #[error("AgentScope execution failed: {0}")]
    Execution(String),
}
