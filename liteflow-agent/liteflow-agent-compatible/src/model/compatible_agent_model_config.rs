// SPDX-License-Identifier: Apache-2.0
//
// LiteFlow-Rust 原创代码：通用兼容模型配置，对接 ProviderToModelAdapter。

use std::sync::Arc;

use agentscope_core::Model;
use liteflow_agent_provider_core::ProviderToModelAdapter;
use serde::{Deserialize, Serialize};

use crate::provider::AuthStyle;

/// 通用 OpenAI 兼容模型配置。
///
/// 适用于 DeepSeek/Kimi/Minimax/Qwen/Groq/Mistral/xAI 等所有遵循
/// `/v1/chat/completions` 格式的服务。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatibleAgentModelConfig {
    /// API 密钥。
    pub api_key: String,
    /// 模型名称。
    pub model_name: String,
    /// 服务基址（如 `https://api.deepseek.com/v1`）。
    pub base_url: String,
    /// provider 名称（用于错误信息）。
    pub provider_name: Option<String>,
    /// 是否使用自定义 auth header（默认 Bearer）。
    pub custom_auth: bool,
    /// 是否支持 native tool calling（默认 true）。
    pub native_tool_calling: bool,
    /// 可选 max_tokens 覆盖。
    pub max_tokens: Option<u32>,
    /// 采样温度。
    pub temperature: Option<f64>,
}

impl CompatibleAgentModelConfig {
    #[must_use]
    pub fn new(
        api_key: impl Into<String>,
        model_name: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Self {
        Self {
            api_key: api_key.into(),
            model_name: model_name.into(),
            base_url: base_url.into(),
            provider_name: None,
            custom_auth: false,
            native_tool_calling: true,
            max_tokens: None,
            temperature: None,
        }
    }

    /// 便捷构造：DeepSeek。
    #[must_use]
    pub fn deepseek(api_key: impl Into<String>, model_name: impl Into<String>) -> Self {
        Self::new(
            api_key,
            model_name,
            crate::provider::presets::DEEPSEEK_BASE_URL,
        )
        .with_provider_name("deepseek")
    }

    /// 便捷构造：Kimi/Moonshot。
    #[must_use]
    pub fn moonshot(api_key: impl Into<String>, model_name: impl Into<String>) -> Self {
        Self::new(
            api_key,
            model_name,
            crate::provider::presets::MOONSHOT_BASE_URL,
        )
        .with_provider_name("moonshot")
    }

    /// 便捷构造：通义千问（兼容模式）。
    #[must_use]
    pub fn qwen(api_key: impl Into<String>, model_name: impl Into<String>) -> Self {
        Self::new(api_key, model_name, crate::provider::presets::QWEN_BASE_URL)
            .with_provider_name("qwen")
    }

    /// 便捷构造：Groq。
    #[must_use]
    pub fn groq(api_key: impl Into<String>, model_name: impl Into<String>) -> Self {
        Self::new(api_key, model_name, crate::provider::presets::GROQ_BASE_URL)
            .with_provider_name("groq")
    }

    #[must_use]
    pub fn with_provider_name(mut self, name: impl Into<String>) -> Self {
        self.provider_name = Some(name.into());
        self
    }

    /// 构建 `Arc<dyn Model>`（不触网）。
    #[must_use]
    pub fn build(&self) -> Arc<dyn Model> {
        let mut provider = crate::provider::CompatibleProvider::new(
            self.provider_name
                .clone()
                .unwrap_or_else(|| "compatible".to_string()),
            self.base_url.clone(),
            Some(&self.api_key),
        );
        if self.custom_auth {
            provider = provider.with_auth_style(AuthStyle::Custom);
        }
        if !self.native_tool_calling {
            provider = provider.with_native_tool_calling(false);
        }
        if let Some(max_tokens) = self.max_tokens {
            provider = provider.with_max_tokens(max_tokens);
        }
        Arc::new(ProviderToModelAdapter::new(
            self.model_name.clone(),
            self.model_name.clone(),
            Arc::new(provider),
            self.temperature.unwrap_or(0.7),
        ))
    }
}
