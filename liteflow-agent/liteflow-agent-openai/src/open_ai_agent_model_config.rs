//! 对应 Java Agent 模型配置中的 OpenAI 及兼容提供商。

use std::sync::Arc;

use agentscope_core::Model;
use agentscope_core::model::openai::OpenAIChatModel;
use serde::{Deserialize, Serialize};

/// OpenAI 及兼容接口模型配置。
///
/// 通过 `base_url` 可连接 DeepSeek、GLM、Kimi、Minimax 等兼容服务。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenAiAgentModelConfig {
    /// API 密钥。
    pub api_key: String,
    /// 模型名称。
    pub model_name: String,
    /// 可选兼容服务地址。
    pub base_url: Option<String>,
    /// 可选 API 路径。
    pub endpoint_path: Option<String>,
    /// 是否请求流式响应。
    pub stream: bool,
}

impl OpenAiAgentModelConfig {
    /// 创建 OpenAI 兼容配置。
    #[must_use]
    pub fn new(api_key: impl Into<String>, model_name: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model_name: model_name.into(),
            base_url: None,
            endpoint_path: None,
            stream: true,
        }
    }

    /// 构建可直接注入 `ReActAgentComponent` 的 AgentScope 模型。
    #[must_use]
    pub fn build(&self) -> Arc<dyn Model> {
        let mut builder = OpenAIChatModel::builder()
            .api_key(&self.api_key)
            .model_name(&self.model_name)
            .stream(self.stream);
        if let Some(base_url) = &self.base_url {
            builder = builder.base_url(base_url);
        }
        if let Some(endpoint_path) = &self.endpoint_path {
            builder = builder.endpoint_path(endpoint_path);
        }
        Arc::new(builder.build())
    }
}
