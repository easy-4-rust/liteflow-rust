// SPDX-License-Identifier: Apache-2.0
//
// LiteFlow-Rust 原创代码：Telnyx 模型配置，对接 ProviderToModelAdapter。

use std::sync::Arc;

use agentscope_core::Model;
use liteflow_agent_provider_core::ProviderToModelAdapter;
use serde::{Deserialize, Serialize};

/// Telnyx AI 推理模型配置。
///
/// 通过 OpenAI 兼容 API 接入 53+ 模型（GPT-4o/Claude/Llama/Mistral 等）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelnyxAgentModelConfig {
    /// Telnyx API 密钥。
    pub api_key: String,
    /// 模型名称，如 `openai/gpt-4o`。
    pub model_name: String,
    /// 采样温度。
    pub temperature: Option<f64>,
}

impl TelnyxAgentModelConfig {
    /// 创建 Telnyx 配置。
    #[must_use]
    pub fn new(api_key: impl Into<String>, model_name: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model_name: model_name.into(),
            temperature: None,
        }
    }

    /// 构建可直接注入 `ReActAgentComponent` 的 AgentScope 模型。
    ///
    /// 不触网：仅构造 provider 与 adapter，实际请求在调用时发生。
    #[must_use]
    pub fn build(&self) -> Arc<dyn Model> {
        let provider = crate::provider::TelnyxProvider::new(Some(&self.api_key));
        Arc::new(ProviderToModelAdapter::new(
            self.model_name.clone(),
            self.model_name.clone(),
            std::sync::Arc::new(provider),
            self.temperature.unwrap_or(0.7),
        ))
    }
}
