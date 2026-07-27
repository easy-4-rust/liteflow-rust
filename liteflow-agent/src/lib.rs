//! LiteFlow Agent 聚合 crate。
//!
//! ReAct 核心与各模型提供商分别位于独立子 crate；本层只负责 Cargo feature
//! 编排和稳定重导出，不定义业务对象。
//!
//! ## 平台覆盖
//!
//! - **agentscope 原生**（5 个）：openai / anthropic / gemini / dashscope
//! - **zeroclaw 衍生**（6 个）：telnyx / glm / copilot / bedrock / openrouter / compatible
//!   - 通用 provider trait 与 `ProviderToModelAdapter` 见 [`provider_core`] 模块。

// ── agentscope 原生平台 ──

#[cfg(feature = "anthropic")]
pub use liteflow_agent_anthropic as anthropic;
#[cfg(feature = "anthropic")]
pub use liteflow_agent_anthropic::{
    Anthropic, AnthropicAgentModelConfig, AnthropicCompatible, AnthropicModelFactory,
    AnthropicSpec, AnthropicThinking,
};
#[cfg(feature = "core")]
pub use liteflow_agent_core as core;
#[cfg(feature = "dashscope")]
pub use liteflow_agent_dashscope as dashscope;
#[cfg(feature = "dashscope")]
pub use liteflow_agent_dashscope::{
    DashScope, DashScopeAgentModelConfig, DashScopeModelFactory, DashScopeSpec, DashScopeThinking,
};
#[cfg(feature = "gemini")]
pub use liteflow_agent_gemini as gemini;
#[cfg(feature = "gemini")]
pub use liteflow_agent_gemini::{
    Gemini, GeminiAgentModelConfig, GeminiModelFactory, GeminiSpec, GeminiThinking,
};
#[cfg(feature = "openai")]
pub use liteflow_agent_openai as openai;
#[cfg(feature = "openai")]
pub use liteflow_agent_openai::{
    DeepSeek, Glm, Kimi, Minimax, OpenAi, OpenAiAgentModelConfig, OpenAiCompatible,
    OpenAiCompatiblePresets, OpenAiCompatibleSpec, OpenAiModelFactory, OpenAiSpec,
};

// ── zeroclaw 衍生平台（provider-core 体系）──

#[cfg(feature = "provider-core")]
pub use liteflow_agent_provider_core as provider_core;
#[cfg(feature = "provider-core")]
pub use liteflow_agent_provider_core::ProviderToModelAdapter;

#[cfg(feature = "telnyx")]
pub use liteflow_agent_telnyx as telnyx;
#[cfg(feature = "telnyx")]
pub use liteflow_agent_telnyx::TelnyxAgentModelConfig;

#[cfg(feature = "glm")]
pub use liteflow_agent_glm as glm;
#[cfg(feature = "glm")]
pub use liteflow_agent_glm::GlmAgentModelConfig;

#[cfg(feature = "openrouter")]
pub use liteflow_agent_openrouter as openrouter;
#[cfg(feature = "openrouter")]
pub use liteflow_agent_openrouter::OpenRouterAgentModelConfig;

#[cfg(feature = "copilot")]
pub use liteflow_agent_copilot as copilot;
#[cfg(feature = "copilot")]
pub use liteflow_agent_copilot::CopilotAgentModelConfig;

#[cfg(feature = "bedrock")]
pub use liteflow_agent_bedrock as bedrock;
#[cfg(feature = "bedrock")]
pub use liteflow_agent_bedrock::BedrockAgentModelConfig;

#[cfg(feature = "compatible")]
pub use liteflow_agent_compatible as compatible;
#[cfg(feature = "compatible")]
pub use liteflow_agent_compatible::CompatibleAgentModelConfig;

// ── 核心类型重导出 ──

#[cfg(feature = "core")]
#[allow(deprecated)]
pub use liteflow_agent_core::{
    AgentConfig, AgentConfigException, AgentDefinition, AgentError, AgentEventType, AgentException,
    AgentInvocationException, AgentSession, AgentSessionFactory, AgentSessionFactoryRegistration,
    AgentSessionFactoryRegistry, AgentSessionManager, CredentialResolver, DefaultsConfig,
    InMemoryAgentSessionFactory, LocalFileAgentSessionFactory, LocalFileMemoryConfig,
    LoggingConfig, MemoryStorageConfig, MemoryStorageMode, ModelSpec, MysqlAgentSessionFactory,
    MysqlMemoryConfig, NoneAgentSessionFactory, PlatformCredential, ReActAgentComponent,
    ReActAgentComponentBuilder, ReActAgentContext, RedisAgentSessionFactory, RedisClientType,
    RedisMemoryConfig, SessionConfig, ShellConfig, ShellMode, SkillsConfig, WorkspaceConfig,
};
