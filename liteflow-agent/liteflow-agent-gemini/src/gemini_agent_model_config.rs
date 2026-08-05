//! 对应 Java Agent 模型配置中的 Google Gemini 提供商。

use std::sync::Arc;

use agentscope_core::Model;
use agentscope_core::model::gemini::GeminiChatModel;
use serde::{Deserialize, Serialize};

/// Google Gemini 模型配置与 AgentScope 构建适配器。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiAgentModelConfig {
    /// Gemini API 密钥。
    pub api_key: String,
    /// 模型名称。
    pub model_name: String,
    /// 可选兼容网关地址。
    pub base_url: Option<String>,
    /// 是否请求流式响应。
    pub stream: bool,
}

impl GeminiAgentModelConfig {
    /// 创建 Gemini 配置。
    #[must_use]
    pub fn new(api_key: impl Into<String>, model_name: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model_name: model_name.into(),
            base_url: None,
            stream: true,
        }
    }

    /// 构建可直接注入 `ReActAgentComponent` 的 AgentScope 模型。
    #[must_use]
    pub fn build(&self) -> Arc<dyn Model> {
        let mut builder = GeminiChatModel::builder()
            .api_key(&self.api_key)
            .model_name(&self.model_name)
            .stream(self.stream);
        if let Some(base_url) = &self.base_url {
            builder = builder.base_url(base_url);
        }
        Arc::new(builder.build())
    }
}
