// SPDX-License-Identifier: Apache-2.0
//
// LiteFlow-Rust Agent Provider Core — 衍生自 ZeroClaw 的 Provider trait 体系。
//
// 本 crate 提供：
// - zeroclaw `Provider` trait 及其消息/响应类型（ChatMessage/ChatResponse/StreamChunk 等）
// - `ProviderToModelAdapter`：把任意 `Provider` 包装成 `agentscope_core::Model`
// - `ToolSpec` / `QuotaMetadata` 等共享类型
// - 简化版代理客户端构建（`proxy` 模块）
//
// 各平台子 crate（liteflow-agent-glm/copilot/bedrock 等）依赖本 crate，
// 复用 Provider trait 与 adapter，避免重复。
//
// 衍生来源：https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// "ZeroClaw" 是 ZeroClaw Labs 的商标；本项目与其无官方关联。

//! LiteFlow Agent Provider Core。
//!
//! 详见 crate 级文档注释。

pub mod adapter;
#[cfg(feature = "auth")]
pub mod auth;
pub mod backoff;
pub mod health;
#[cfg(feature = "multimodal")]
pub mod multimodal;
#[cfg(feature = "codex")]
pub mod openai_codex;
pub mod proxy;
pub mod quota_adapter;
pub mod quota_types;
pub mod reliable;
pub mod router;
pub mod runtime_options;
pub mod tool_spec;
pub mod traits;
pub mod util;

pub use adapter::ProviderToModelAdapter;
pub use backoff::{BackoffEntry, BackoffStore};
pub use health::{ProviderHealthState, ProviderHealthTracker};
#[cfg(feature = "multimodal")]
pub use multimodal::{
    MultimodalConfig, MultimodalError, PreparedMessages, contains_image_markers,
    count_image_markers, extract_ollama_image_payload, parse_image_markers,
    prepare_messages_for_provider,
};
pub use proxy::ProxyConfig;
pub use quota_adapter::{
    AnthropicQuotaExtractor, GeminiQuotaExtractor, OpenAIQuotaExtractor, QuotaExtractor,
    UniversalQuotaExtractor,
};
pub use quota_types::{
    ProfileQuotaInfo, ProviderQuotaInfo, QuotaMetadata, QuotaStatus, QuotaSummary,
};
pub use reliable::ReliableProvider;
pub use router::{Route, RouterProvider};
pub use runtime_options::ProviderRuntimeOptions;
pub use tool_spec::{ToolResult, ToolSpec};
pub use traits::{
    ChatMessage, ChatRequest, ChatResponse, ConversationMessage, Provider, ProviderCapabilities,
    ProviderCapabilityError, StreamChunk, StreamError, StreamOptions, StreamResult, TokenUsage,
    ToolCall, ToolResultMessage, ToolsPayload, build_tool_instructions_text,
};
pub use util::{api_error, sanitize_api_error, scrub_secret_patterns};
