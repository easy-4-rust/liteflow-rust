use std::sync::Arc;

use agentscope_core::Model;
use agentscope_core::model::GenerateOptions;
use agentscope_core::model::gemini::GeminiChatModel;
use liteflow_agent_core::{
    AgentConfig, AgentConfigException, CredentialResolver, ModelSpec, PlatformCredential,
};

use super::GeminiThinking;

/// Gemini 模型描述符，组合共性参数和 thinking level/budget。
///
/// 对应 Java: `com.yomahub.liteflow.agent.gemini.GeminiSpec`。
#[derive(Debug, Clone, PartialEq)]
pub struct GeminiSpec {
    model_name: String,
    common: ModelSpec,
    thinking_level: Option<String>,
    thinking_budget: Option<u32>,
}

impl GeminiSpec {
    /// 使用模型名称创建 Gemini 描述符。
    ///
    /// 对应 Java: `GeminiSpec#GeminiSpec`。
    #[must_use]
    pub fn new(model_name: impl Into<String>) -> Self {
        Self {
            model_name: model_name.into(),
            common: ModelSpec::new(),
            thinking_level: None,
            thinking_budget: None,
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

    /// 设置缓存控制；Java Gemini resolve 当前不消费该字段，但仍保留基类状态。
    ///
    /// 对应 Java: `ModelSpec#cacheControl`。
    #[must_use]
    pub fn cache_control(mut self, value: bool) -> Self {
        self.common = self.common.cache_control(value);
        self
    }

    /// 使用子构建器配置 Gemini thinking。
    ///
    /// 对应 Java: `GeminiSpec#thinking`。
    #[must_use]
    pub fn thinking<F>(mut self, configure: F) -> Self
    where
        F: FnOnce(&mut GeminiThinking),
    {
        let mut thinking = GeminiThinking::default();
        configure(&mut thinking);
        self.thinking_level = thinking.get_level().map(ToOwned::to_owned);
        self.thinking_budget = thinking.get_budget();
        self
    }

    /// 返回模型名称。对应 Java: `GeminiSpec#getModelName`。
    #[must_use]
    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    /// 返回 thinking 等级。对应 Java: `GeminiSpec#getThinkingLevel`。
    #[must_use]
    pub fn thinking_level(&self) -> Option<&str> {
        self.thinking_level.as_deref()
    }

    /// 返回 thinking 预算。对应 Java: `GeminiSpec#getThinkingBudget`。
    #[must_use]
    pub fn thinking_budget(&self) -> Option<u32> {
        self.thinking_budget
    }

    /// 从 `AgentConfig.gemini` 解析凭证并构造真实 AgentScope Gemini 模型。
    ///
    /// 对应 Java: `GeminiSpec#resolve`。
    pub fn resolve(&self, config: &AgentConfig) -> Result<Arc<dyn Model>, AgentConfigException> {
        let credential = CredentialResolver::require_first_class(
            Some(config.gemini()),
            "liteflow.agent.gemini",
        )?;
        Ok(self.build_model(credential))
    }

    fn build_model(&self, credential: &PlatformCredential) -> Arc<dyn Model> {
        let mut builder = GeminiChatModel::builder()
            .api_key(credential.api_key().unwrap_or_default())
            .model_name(&self.model_name);
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
            && self.thinking_level.is_none()
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
            reasoning_effort: self.thinking_level.clone(),
            thinking_budget: self.thinking_budget,
            ..GenerateOptions::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use liteflow_agent_core::{AgentConfig, PlatformCredential};

    use super::GeminiSpec;

    #[test]
    fn thinking_resolution_builds_model_without_network_io() {
        let spec = GeminiSpec::new("gemini-test")
            .thinking(|thinking| {
                thinking.level("high").budget(2_048);
            })
            .stream(false);
        assert_eq!(spec.thinking_level(), Some("high"));
        assert_eq!(spec.thinking_budget(), Some(2_048));

        let mut config = AgentConfig::default();
        let mut credential = PlatformCredential::default();
        credential.set_api_key(Some("test-key".to_string()));
        config.set_gemini(credential);
        let model = spec.resolve(&config).expect("构造模型不应触网");
        assert_eq!(model.name(), "gemini-test");
    }
}
