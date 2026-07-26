//! LiteFlow Agent 聚合 crate。
//!
//! ReAct 核心与各模型提供商分别位于独立子 crate；本层只负责 Cargo feature
//! 编排和稳定重导出，不定义业务对象。

#[cfg(feature = "anthropic")]
pub use liteflow_agent_anthropic as anthropic;
#[cfg(feature = "anthropic")]
pub use liteflow_agent_anthropic::AnthropicAgentModelConfig;
#[cfg(feature = "core")]
pub use liteflow_agent_core as core;
#[cfg(feature = "dashscope")]
pub use liteflow_agent_dashscope as dashscope;
#[cfg(feature = "dashscope")]
pub use liteflow_agent_dashscope::DashScopeAgentModelConfig;
#[cfg(feature = "gemini")]
pub use liteflow_agent_gemini as gemini;
#[cfg(feature = "gemini")]
pub use liteflow_agent_gemini::GeminiAgentModelConfig;
#[cfg(feature = "openai")]
pub use liteflow_agent_openai as openai;
#[cfg(feature = "openai")]
pub use liteflow_agent_openai::OpenAiAgentModelConfig;

#[cfg(feature = "core")]
#[allow(deprecated)]
pub use liteflow_agent_core::{
    AgentConfig, AgentDefinition, AgentError, AgentEventType, AgentMemoryMode, AgentSessionManager,
    DefaultsConfig, LocalFileMemoryConfig, LoggingConfig, MemoryStorageConfig, MemoryStorageMode,
    MysqlMemoryConfig, PlatformCredential, ReActAgentComponent, ReActAgentComponentBuilder,
    RedisClientType, RedisMemoryConfig, SessionConfig, ShellConfig, ShellMode, SkillsConfig,
    WorkspaceConfig,
};
