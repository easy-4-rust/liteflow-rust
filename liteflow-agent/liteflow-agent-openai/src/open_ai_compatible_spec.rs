use std::sync::Arc;

use agentscope_core::Model;
use liteflow_agent_core::{AgentConfig, AgentConfigException, CredentialResolver};

use super::OpenAiSpec;

/// OpenAI 兼容平台描述符，凭证来自兼容平台 Map，并支持内置默认 base URL。
///
/// 对应 Java: `com.yomahub.liteflow.agent.openai.OpenAICompatibleSpec`。
#[derive(Debug, Clone, PartialEq)]
pub struct OpenAiCompatibleSpec {
    config_key: String,
    default_base_url: Option<String>,
    open_ai: OpenAiSpec,
}

impl OpenAiCompatibleSpec {
    /// 创建 OpenAI 兼容平台描述符。
    ///
    /// 对应 Java: `OpenAICompatibleSpec#OpenAICompatibleSpec`。
    #[must_use]
    pub fn new(
        config_key: impl Into<String>,
        model_name: impl Into<String>,
        default_base_url: Option<impl Into<String>>,
    ) -> Self {
        Self {
            config_key: config_key.into(),
            default_base_url: default_base_url.map(Into::into),
            open_ai: OpenAiSpec::new(model_name),
        }
    }

    /// 设置采样温度。对应 Java: `ModelSpec#temperature`。
    #[must_use]
    pub fn temperature(mut self, value: f64) -> Self {
        self.open_ai = self.open_ai.temperature(value);
        self
    }

    /// 设置核采样概率。对应 Java: `ModelSpec#topP`。
    #[must_use]
    pub fn top_p(mut self, value: f64) -> Self {
        self.open_ai = self.open_ai.top_p(value);
        self
    }

    /// 设置 Top-K。对应 Java: `ModelSpec#topK`。
    #[must_use]
    pub fn top_k(mut self, value: u32) -> Self {
        self.open_ai = self.open_ai.top_k(value);
        self
    }

    /// 设置最大输出 token 数。对应 Java: `ModelSpec#maxTokens`。
    #[must_use]
    pub fn max_tokens(mut self, value: u32) -> Self {
        self.open_ai = self.open_ai.max_tokens(value);
        self
    }

    /// 设置随机种子。对应 Java: `ModelSpec#seed`。
    #[must_use]
    pub fn seed(mut self, value: i64) -> Self {
        self.open_ai = self.open_ai.seed(value);
        self
    }

    /// 设置流式响应开关。对应 Java: `ModelSpec#stream`。
    #[must_use]
    pub fn stream(mut self, value: bool) -> Self {
        self.open_ai = self.open_ai.stream(value);
        self
    }

    /// 设置提示缓存控制。对应 Java: `ModelSpec#cacheControl`。
    #[must_use]
    pub fn cache_control(mut self, value: bool) -> Self {
        self.open_ai = self.open_ai.cache_control(value);
        self
    }

    /// 设置推理力度。对应 Java: `OpenAISpec#reasoningEffort`。
    #[must_use]
    pub fn reasoning_effort(mut self, level: impl Into<String>) -> Self {
        self.open_ai = self.open_ai.reasoning_effort(level);
        self
    }

    /// 设置频率惩罚。对应 Java: `OpenAISpec#frequencyPenalty`。
    #[must_use]
    pub fn frequency_penalty(mut self, value: f64) -> Self {
        self.open_ai = self.open_ai.frequency_penalty(value);
        self
    }

    /// 设置存在惩罚。对应 Java: `OpenAISpec#presencePenalty`。
    #[must_use]
    pub fn presence_penalty(mut self, value: f64) -> Self {
        self.open_ai = self.open_ai.presence_penalty(value);
        self
    }

    /// 从兼容平台配置 Map 解析凭证并构造真实 AgentScope 模型。
    ///
    /// 用户配置的非空 base URL 优先于描述符内置默认值。
    ///
    /// 对应 Java: `OpenAICompatibleSpec#resolve`。
    pub fn resolve(&self, config: &AgentConfig) -> Result<Arc<dyn Model>, AgentConfigException> {
        let credential = CredentialResolver::require_compatible(
            Some(config.openai_compatible()),
            &self.config_key,
            "liteflow.agent.openai-compatible",
        )?;
        let base_url = credential
            .base_url()
            .filter(|value| !value.trim().is_empty())
            .or(self.default_base_url.as_deref());
        Ok(self
            .open_ai
            .build_model(credential.api_key().unwrap_or_default(), base_url))
    }
}
