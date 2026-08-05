//! 对应 Java Agent 模型配置中的 DashScope/通义提供商。

use std::sync::Arc;

use agentscope_core::Model;
use agentscope_core::model::dashscope::DashScopeChatModel;
use serde::{Deserialize, Serialize};

/// 阿里云 DashScope 模型配置与 AgentScope 构建适配器。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashScopeAgentModelConfig {
    /// DashScope API 密钥。
    pub api_key: String,
    /// 模型名称，如 `qwen-plus`。
    pub model_name: String,
    /// 可选兼容网关地址。
    pub base_url: Option<String>,
    /// 是否请求流式响应。
    pub stream: bool,
    /// 是否开启思考模式。
    pub enable_thinking: bool,
    /// 是否开启联网搜索。
    pub enable_search: bool,
}

impl DashScopeAgentModelConfig {
    /// 创建 DashScope 配置。
    #[must_use]
    pub fn new(api_key: impl Into<String>, model_name: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model_name: model_name.into(),
            base_url: None,
            stream: true,
            enable_thinking: false,
            enable_search: false,
        }
    }

    /// 构建可直接注入 `ReActAgentComponent` 的 AgentScope 模型。
    #[must_use]
    pub fn build(&self) -> Arc<dyn Model> {
        let mut builder = DashScopeChatModel::builder()
            .api_key(&self.api_key)
            .model_name(&self.model_name)
            .stream(self.stream)
            .enable_thinking(self.enable_thinking)
            .enable_search(self.enable_search);
        if let Some(base_url) = &self.base_url {
            builder = builder.base_url(base_url);
        }
        Arc::new(builder.build())
    }
}
