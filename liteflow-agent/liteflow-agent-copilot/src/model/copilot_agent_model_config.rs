// SPDX-License-Identifier: Apache-2.0
//
// LiteFlow-Rust 原创代码：Copilot 模型配置，对接 ProviderToModelAdapter。

use std::sync::Arc;

use agentscope_core::Model;
use liteflow_agent_provider_core::ProviderToModelAdapter;
use serde::{Deserialize, Serialize};

/// GitHub Copilot 模型配置（OAuth 设备流认证）。
///
/// 首次使用时会在终端提示访问 github.com/login/device 完成授权，
/// token 缓存到本地并自动刷新。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopilotAgentModelConfig {
    /// 可选的 GitHub access token（为空时走设备码流）。
    pub github_token: Option<String>,
    /// 模型名称，如 `gpt-4o`。
    pub model_name: String,
    /// 采样温度。
    pub temperature: Option<f64>,
}

impl CopilotAgentModelConfig {
    #[must_use]
    pub fn new(github_token: Option<impl Into<String>>, model_name: impl Into<String>) -> Self {
        Self {
            github_token: github_token.map(Into::into),
            model_name: model_name.into(),
            temperature: None,
        }
    }

    /// 构建 `Arc<dyn Model>`（不触网：OAuth 在首次请求时才发起）。
    #[must_use]
    pub fn build(&self) -> Arc<dyn Model> {
        let provider = crate::provider::CopilotProvider::new(self.github_token.as_deref());
        Arc::new(ProviderToModelAdapter::new(
            self.model_name.clone(),
            self.model_name.clone(),
            Arc::new(provider),
            self.temperature.unwrap_or(0.7),
        ))
    }
}
