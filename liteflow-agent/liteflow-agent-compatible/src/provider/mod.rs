mod auth_style;
mod compatible_provider;

pub use auth_style::AuthStyle;
pub use compatible_provider::CompatibleProvider;

/// 常用 OpenAI 兼容服务的 API 基址。
pub mod presets {
    pub use super::compatible_provider::presets::{
        DEEPSEEK_BASE_URL, GLM_BASE_URL, GROQ_BASE_URL, MINIMAX_BASE_URL, MISTRAL_BASE_URL,
        MOONSHOT_BASE_URL, QWEN_BASE_URL, XAI_BASE_URL,
    };
}
