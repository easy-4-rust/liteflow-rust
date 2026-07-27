//! LiteFlow Agent 通用 OpenAI 兼容模型适配（衍生自 ZeroClaw，Apache-2.0）。
//!
//! 适用于 DeepSeek/Kimi/Minimax/Qwen/Groq/Mistral/xAI 等。

pub mod model;
pub mod provider;

pub use model::CompatibleAgentModelConfig;
pub use provider::{AuthStyle, CompatibleProvider};
