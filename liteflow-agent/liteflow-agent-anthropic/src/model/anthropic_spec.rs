use std::sync::Arc;

use agentscope_core::Model;
use agentscope_core::model::GenerateOptions;
use agentscope_core::model::anthropic::AnthropicChatModel;
use liteflow_agent_core::{
    AgentConfig, AgentConfigException, CredentialResolver, ModelSpec, PlatformCredential,
};

use super::AnthropicThinking;

/// Anthropic 模型描述符，支持头等平台与 anthropic-compatible 凭证来源。
///
/// 对应 Java: `com.yomahub.liteflow.agent.anthropic.AnthropicSpec`。
#[derive(Debug, Clone, PartialEq)]
pub struct AnthropicSpec {
    model_name: String,
    common: ModelSpec,
    thinking_budget: Option<u32>,
    thinking_enabled: Option<bool>,
    compatible_config_key: Option<String>,
}

impl AnthropicSpec {
    /// 创建使用头等 Anthropic 凭证的描述符。
    ///
    /// 对应 Java: `AnthropicSpec#AnthropicSpec(String)`。
    #[must_use]
    pub fn new(model_name: impl Into<String>) -> Self {
        Self::with_compatible_key(model_name, None::<String>)
    }

    /// 创建可选择兼容配置 key 的描述符。
    ///
    /// `None` 读取 `AgentConfig.anthropic`，`Some` 读取
    /// `AgentConfig.anthropic_compatible`。
    ///
    /// 对应 Java: `AnthropicSpec#AnthropicSpec(String, String)`。
    #[must_use]
    pub fn with_compatible_key(
        model_name: impl Into<String>,
        compatible_config_key: Option<impl Into<String>>,
    ) -> Self {
        Self {
            model_name: model_name.into(),
            common: ModelSpec::new(),
            thinking_budget: None,
            thinking_enabled: None,
            compatible_config_key: compatible_config_key.map(Into::into),
        }
    }

    /// 设置采样温度。对应 Java: `ModelSpec#temperature`。
    #[must_use]
    pub fn temperature(mut self, value: f64) -> Self {
        self.common = self.common.temperature(value);
        self
    }

    /// 设置核采样概率。对应 Java: `ModelSpec#topP`。
    #[must_use]
    pub fn top_p(mut self, value: f64) -> Self {
        self.common = self.common.top_p(value);
        self
    }

    /// 设置 Top-K。对应 Java: `ModelSpec#topK`。
    #[must_use]
    pub fn top_k(mut self, value: u32) -> Self {
        self.common = self.common.top_k(value);
        self
    }

    /// 设置最大输出 token 数。对应 Java: `ModelSpec#maxTokens`。
    #[must_use]
    pub fn max_tokens(mut self, value: u32) -> Self {
        self.common = self.common.max_tokens(value);
        self
    }

    /// 设置随机种子。对应 Java: `ModelSpec#seed`。
    #[must_use]
    pub fn seed(mut self, value: i64) -> Self {
        self.common = self.common.seed(value);
        self
    }

    /// 设置流式响应开关。对应 Java: `ModelSpec#stream`。
    #[must_use]
    pub fn stream(mut self, value: bool) -> Self {
        self.common = self.common.stream(value);
        self
    }

    /// 设置缓存控制；Java Anthropic resolve 当前不消费该字段，但仍保留基类状态。
    ///
    /// 对应 Java: `ModelSpec#cacheControl`。
    #[must_use]
    pub fn cache_control(mut self, value: bool) -> Self {
        self.common = self.common.cache_control(value);
        self
    }

    /// 使用子构建器配置 Anthropic thinking。
    ///
    /// 对应 Java: `AnthropicSpec#thinking`。
    #[must_use]
    pub fn thinking<F>(mut self, configure: F) -> Self
    where
        F: FnOnce(&mut AnthropicThinking),
    {
        let mut thinking = AnthropicThinking::default();
        configure(&mut thinking);
        self.thinking_budget = thinking.get_budget();
        self.thinking_enabled = thinking.get_enabled();
        self
    }

    /// 返回模型名称。对应 Java: `AnthropicSpec#getModelName`。
    #[must_use]
    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    /// 返回 thinking 预算。对应 Java: `AnthropicSpec#getThinkingBudget`。
    #[must_use]
    pub fn thinking_budget(&self) -> Option<u32> {
        self.thinking_budget
    }

    /// 返回 thinking 启用状态。对应 Java: `AnthropicSpec#getThinkingEnabled`。
    #[must_use]
    pub fn thinking_enabled(&self) -> Option<bool> {
        self.thinking_enabled
    }

    /// 从 Agent 配置解析凭证并构造真实 AgentScope Anthropic 模型。
    ///
    /// 对应 Java: `AnthropicSpec#resolve`。
    pub fn resolve(&self, config: &AgentConfig) -> Result<Arc<dyn Model>, AgentConfigException> {
        let credential = if let Some(key) = &self.compatible_config_key {
            CredentialResolver::require_compatible(
                Some(config.anthropic_compatible()),
                key,
                "liteflow.agent.anthropic-compatible",
            )?
        } else {
            CredentialResolver::require_first_class(
                Some(config.anthropic()),
                "liteflow.agent.anthropic",
            )?
        };
        Ok(self.build_model(credential))
    }

    fn build_model(&self, credential: &PlatformCredential) -> Arc<dyn Model> {
        let mut builder = AnthropicChatModel::builder()
            .api_key(credential.api_key().unwrap_or_default())
            .model_name(&self.model_name);
        if let Some(base_url) = credential
            .base_url()
            .filter(|value| !value.trim().is_empty())
        {
            builder = builder.base_url(base_url);
        }
        if let Some(options) = self.generate_options() {
            builder = builder.default_options(options);
        }
        if let Some(stream) = self.common.get_stream() {
            builder = builder.stream(stream);
        }
        Arc::new(builder.build())
    }

    fn generate_options(&self) -> Option<GenerateOptions> {
        if self.common.get_temperature().is_none()
            && self.common.get_top_p().is_none()
            && self.common.get_top_k().is_none()
            && self.common.get_max_tokens().is_none()
            && self.common.get_seed().is_none()
            && self.thinking_budget.is_none()
        {
            return None;
        }
        Some(GenerateOptions {
            temperature: self.common.get_temperature(),
            top_p: self.common.get_top_p(),
            top_k: self.common.get_top_k(),
            max_tokens: self.common.get_max_tokens(),
            seed: self.common.get_seed(),
            thinking_budget: self.thinking_budget,
            ..GenerateOptions::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use liteflow_agent_core::{AgentConfig, PlatformCredential};

    use super::AnthropicSpec;

    #[test]
    fn thinking_and_first_class_resolution_build_without_network_io() {
        let spec = AnthropicSpec::new("claude-test")
            .thinking(|thinking| {
                thinking.budget(1_024).enabled(true);
            })
            .stream(false);
        assert_eq!(spec.thinking_budget(), Some(1_024));
        assert_eq!(spec.thinking_enabled(), Some(true));

        let mut config = AgentConfig::default();
        let mut credential = PlatformCredential::default();
        credential.set_api_key(Some("test-key".to_string()));
        credential.set_base_url(Some("https://example.invalid".to_string()));
        config.set_anthropic(credential);
        let model = spec.resolve(&config).expect("构造模型不应触网");
        assert_eq!(model.name(), "claude-test");
    }
}
