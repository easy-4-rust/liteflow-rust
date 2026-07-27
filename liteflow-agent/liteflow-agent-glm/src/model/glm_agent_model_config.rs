// SPDX-License-Identifier: Apache-2.0
//
// LiteFlow-Rust 原创代码：GLM 模型配置，对接 ProviderToModelAdapter。

use std::sync::Arc;

use agentscope_core::Model;
use liteflow_agent_provider_core::ProviderToModelAdapter;
use serde::{Deserialize, Serialize};

/// 智谱 GLM 模型配置（JWT 原生认证）。
///
/// API key 格式为 `id.secret`，由 provider 在首次请求时生成 JWT。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlmAgentModelConfig {
    /// GLM API Key，格式 `id.secret`。
    pub api_key: String,
    /// 模型名称，如 `glm-4.6`。
    pub model_name: String,
    /// 可选网关地址（默认 `https://api.z.ai/api/paas/v4`）。
    pub base_url: Option<String>,
    /// 采样温度。
    pub temperature: Option<f64>,
}

impl GlmAgentModelConfig {
    /// 创建 GLM 配置。
    #[must_use]
    pub fn new(api_key: impl Into<String>, model_name: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model_name: model_name.into(),
            base_url: None,
            temperature: None,
        }
    }

    /// 构建 `Arc<dyn Model>`（不触网：JWT 在首次请求时才生成）。
    #[must_use]
    pub fn build(&self) -> Arc<dyn Model> {
        let provider = match &self.base_url {
            Some(url) => crate::provider::GlmProvider::with_base_url(&self.api_key, url.clone()),
            None => crate::provider::GlmProvider::new(&self.api_key),
        };
        Arc::new(ProviderToModelAdapter::new(
            self.model_name.clone(),
            self.model_name.clone(),
            Arc::new(provider),
            self.temperature.unwrap_or(0.7),
        ))
    }
}
