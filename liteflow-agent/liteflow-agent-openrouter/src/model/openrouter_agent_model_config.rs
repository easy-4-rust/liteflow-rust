// SPDX-License-Identifier: Apache-2.0
//
// LiteFlow-Rust 原创代码：OpenRouter 模型配置，对接 ProviderToModelAdapter。

use std::sync::Arc;

use agentscope_core::Model;
use liteflow_agent_provider_core::ProviderToModelAdapter;
use serde::{Deserialize, Serialize};

/// OpenRouter 聚合网关模型配置。
///
/// 通过一个 API key 接入 100+ 模型（如 `anthropic/claude-3.5-sonnet`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenRouterAgentModelConfig {
    /// OpenRouter API 密钥。
    pub api_key: String,
    /// 模型名称，如 `anthropic/claude-3.5-sonnet`。
    pub model_name: String,
    /// 可选 max_tokens 覆盖。
    pub max_tokens: Option<u32>,
    /// 采样温度。
    pub temperature: Option<f64>,
}

impl OpenRouterAgentModelConfig {
    #[must_use]
    pub fn new(api_key: impl Into<String>, model_name: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model_name: model_name.into(),
            max_tokens: None,
            temperature: None,
        }
    }

    /// 构建 `Arc<dyn Model>`（不触网）。
    #[must_use]
    pub fn build(&self) -> Arc<dyn Model> {
        let provider = crate::provider::OpenRouterProvider::new_with_max_tokens(
            Some(&self.api_key),
            self.max_tokens,
        );
        Arc::new(ProviderToModelAdapter::new(
            self.model_name.clone(),
            self.model_name.clone(),
            Arc::new(provider),
            self.temperature.unwrap_or(0.7),
        ))
    }
}
