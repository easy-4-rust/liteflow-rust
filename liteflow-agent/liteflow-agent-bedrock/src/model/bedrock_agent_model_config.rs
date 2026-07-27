// SPDX-License-Identifier: Apache-2.0
//
// LiteFlow-Rust 原创代码：Bedrock 模型配置，对接 ProviderToModelAdapter。

use std::sync::Arc;

use agentscope_core::Model;
use liteflow_agent_provider_core::ProviderToModelAdapter;
use serde::{Deserialize, Serialize};

use crate::provider::AwsCredentials;

/// AWS Bedrock 模型配置（SigV4 签名）。
///
/// 通过 AWS Converse API 接入 Claude（Bedrock 版）、Titan、Llama 等。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BedrockAgentModelConfig {
    /// AWS Access Key ID。
    pub access_key_id: String,
    /// AWS Secret Access Key。
    pub secret_access_key: String,
    /// AWS 区域，如 `us-east-1`。
    pub region: String,
    /// 可选 STS 会话 token。
    pub session_token: Option<String>,
    /// 模型 ID，如 `anthropic.claude-3-5-sonnet-20241022-v2:0`。
    pub model_id: String,
    /// 最大 token 数（默认 4096）。
    pub max_tokens: Option<u32>,
    /// 采样温度。
    pub temperature: Option<f64>,
}

impl BedrockAgentModelConfig {
    #[must_use]
    pub fn new(
        access_key_id: impl Into<String>,
        secret_access_key: impl Into<String>,
        region: impl Into<String>,
        model_id: impl Into<String>,
    ) -> Self {
        Self {
            access_key_id: access_key_id.into(),
            secret_access_key: secret_access_key.into(),
            region: region.into(),
            session_token: None,
            model_id: model_id.into(),
            max_tokens: None,
            temperature: None,
        }
    }

    /// 构建 `Arc<dyn Model>`（不触网：仅构造 provider，签名在请求时计算）。
    #[must_use]
    pub fn build(&self) -> Arc<dyn Model> {
        let mut cred =
            AwsCredentials::new(&self.access_key_id, &self.secret_access_key, &self.region);
        if let Some(token) = &self.session_token {
            cred = cred.with_session_token(token);
        }
        let mut provider = crate::provider::BedrockProvider::with_credentials(cred);
        if let Some(max_tokens) = self.max_tokens {
            provider = provider.with_max_tokens(max_tokens);
        }
        Arc::new(ProviderToModelAdapter::new(
            self.model_id.clone(),
            self.model_id.clone(),
            Arc::new(provider),
            self.temperature.unwrap_or(0.7),
        ))
    }
}
