use std::sync::Arc;

use agentscope_core::Model;
use agentscope_core::model::GenerateOptions;
use agentscope_core::model::openai::OpenAIChatModel;
use liteflow_agent_core::{AgentConfig, AgentConfigException, CredentialResolver, ModelSpec};

/// OpenAI 系模型描述符，组合共性参数与 OpenAI 个性生成参数。
///
/// 对应 Java: `com.yomahub.liteflow.agent.openai.OpenAISpec`。
#[derive(Debug, Clone, PartialEq)]
pub struct OpenAiSpec {
    model_name: String,
    common: ModelSpec,
    reasoning_effort: Option<String>,
    frequency_penalty: Option<f64>,
    presence_penalty: Option<f64>,
}

impl OpenAiSpec {
    /// 使用模型名称创建 OpenAI 描述符。
    ///
    /// 对应 Java: `OpenAISpec#OpenAISpec`。
    #[must_use]
    pub fn new(model_name: impl Into<String>) -> Self {
        Self {
            model_name: model_name.into(),
            common: ModelSpec::new(),
            reasoning_effort: None,
            frequency_penalty: None,
            presence_penalty: None,
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

    /// 设置提示缓存控制。对应 Java: `ModelSpec#cacheControl`。
    #[must_use]
    pub fn cache_control(mut self, value: bool) -> Self {
        self.common = self.common.cache_control(value);
        self
    }

    /// 设置 OpenAI 推理力度。对应 Java: `OpenAISpec#reasoningEffort`。
    #[must_use]
    pub fn reasoning_effort(mut self, level: impl Into<String>) -> Self {
        self.reasoning_effort = Some(level.into());
        self
    }

    /// 设置频率惩罚。对应 Java: `OpenAISpec#frequencyPenalty`。
    #[must_use]
    pub fn frequency_penalty(mut self, value: f64) -> Self {
        self.frequency_penalty = Some(value);
        self
    }

    /// 设置存在惩罚。对应 Java: `OpenAISpec#presencePenalty`。
    #[must_use]
    pub fn presence_penalty(mut self, value: f64) -> Self {
        self.presence_penalty = Some(value);
        self
    }

    /// 返回模型名称。对应 Java: `OpenAISpec#getModelName`。
    #[must_use]
    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    /// 返回共性模型参数。
    #[must_use]
    pub fn common(&self) -> &ModelSpec {
        &self.common
    }

    /// 返回推理力度。对应 Java: `OpenAISpec#getReasoningEffort`。
    #[must_use]
    pub fn reasoning_effort_value(&self) -> Option<&str> {
        self.reasoning_effort.as_deref()
    }

    /// 返回频率惩罚。对应 Java: `OpenAISpec#getFrequencyPenalty`。
    #[must_use]
    pub fn frequency_penalty_value(&self) -> Option<f64> {
        self.frequency_penalty
    }

    /// 返回存在惩罚。对应 Java: `OpenAISpec#getPresencePenalty`。
    #[must_use]
    pub fn presence_penalty_value(&self) -> Option<f64> {
        self.presence_penalty
    }

    /// 从 `AgentConfig.openai` 解析凭证并构造真实 AgentScope 模型。
    ///
    /// 对应 Java: `OpenAISpec#resolve`。
    pub fn resolve(&self, config: &AgentConfig) -> Result<Arc<dyn Model>, AgentConfigException> {
        let credential = CredentialResolver::require_first_class(
            Some(config.openai()),
            "liteflow.agent.openai",
        )?;
        Ok(self.build_model(
            credential.api_key().unwrap_or_default(),
            credential.base_url(),
        ))
    }

    pub(crate) fn build_model(&self, api_key: &str, base_url: Option<&str>) -> Arc<dyn Model> {
        let mut builder = OpenAIChatModel::builder()
            .api_key(api_key)
            .model_name(&self.model_name);
        if let Some(base_url) = base_url.filter(|value| !value.trim().is_empty()) {
            builder = builder.base_url(base_url);
        }
        if let Some(options) = self.generate_options() {
            builder = builder.generate_options(options);
        }
        if let Some(stream) = self.common.get_stream() {
            builder = builder.stream(stream);
        }
        Arc::new(builder.build())
    }

    fn generate_options(&self) -> Option<GenerateOptions> {
        let has_open_ai_options = self.reasoning_effort.is_some()
            || self.frequency_penalty.is_some()
            || self.presence_penalty.is_some();
        let mut options = match (self.common.generate_options(), has_open_ai_options) {
            (None, false) => return None,
            (Some(options), _) => options,
            (None, true) => GenerateOptions::default(),
        };
        options.reasoning_effort.clone_from(&self.reasoning_effort);
        options.frequency_penalty = self.frequency_penalty;
        options.presence_penalty = self.presence_penalty;
        Some(options)
    }
}

#[cfg(test)]
mod tests {
    use liteflow_agent_core::{AgentConfig, PlatformCredential};

    use super::OpenAiSpec;

    #[test]
    fn resolve_requires_config_credential_and_builds_without_network_io() {
        let error = match OpenAiSpec::new("gpt-test").resolve(&AgentConfig::default()) {
            Ok(_) => panic!("空配置必须拒绝"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("liteflow.agent.openai.api-key"));

        let mut config = AgentConfig::default();
        let mut credential = PlatformCredential::default();
        credential.set_api_key(Some("test-key".to_string()));
        credential.set_base_url(Some("https://example.invalid/v1".to_string()));
        config.set_openai(credential);
        let model = OpenAiSpec::new("gpt-test")
            .temperature(0.1)
            .reasoning_effort("low")
            .stream(false)
            .resolve(&config)
            .expect("模型构造不应触网");
        assert_eq!(model.name(), "gpt-test");
    }
}
