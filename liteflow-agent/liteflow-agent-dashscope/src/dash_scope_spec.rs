use std::sync::Arc;

use agentscope_core::Model;
use agentscope_core::model::GenerateOptions;
use agentscope_core::model::dashscope::DashScopeChatModel;
use liteflow_agent_core::{
    AgentConfig, AgentConfigException, CredentialResolver, ModelSpec, PlatformCredential,
};

use super::DashScopeThinking;

/// DashScope 模型描述符，组合共性参数和 thinking budget。
///
/// 对应 Java: `com.yomahub.liteflow.agent.dashscope.DashScopeSpec`。
#[derive(Debug, Clone, PartialEq)]
pub struct DashScopeSpec {
    model_name: String,
    common: ModelSpec,
    thinking_budget: Option<u32>,
}

impl DashScopeSpec {
    /// 使用模型名称创建 DashScope 描述符。
    ///
    /// 对应 Java: `DashScopeSpec#DashScopeSpec`。
    #[must_use]
    pub fn new(model_name: impl Into<String>) -> Self {
        Self {
            model_name: model_name.into(),
            common: ModelSpec::new(),
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

    /// 设置提示缓存控制。对应 Java: `ModelSpec#cacheControl`。
    #[must_use]
    pub fn cache_control(mut self, value: bool) -> Self {
        self.common = self.common.cache_control(value);
        self
    }

    /// 使用子构建器配置 DashScope thinking。
    ///
    /// 对应 Java: `DashScopeSpec#thinking`。
    #[must_use]
    pub fn thinking<F>(mut self, configure: F) -> Self
    where
        F: FnOnce(&mut DashScopeThinking),
    {
        let mut thinking = DashScopeThinking::default();
        configure(&mut thinking);
        self.thinking_budget = thinking.get_budget();
        self
    }

    /// 返回模型名称。对应 Java: `DashScopeSpec#getModelName`。
    #[must_use]
    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    /// 返回 thinking 预算。对应 Java: `DashScopeSpec#getThinkingBudget`。
    #[must_use]
    pub fn thinking_budget(&self) -> Option<u32> {
        self.thinking_budget
    }

    /// 从 `AgentConfig.dashscope` 解析凭证并构造真实 AgentScope 模型。
    ///
    /// 对应 Java: `DashScopeSpec#resolve`。
    pub fn resolve(&self, config: &AgentConfig) -> Result<Arc<dyn Model>, AgentConfigException> {
        let credential = CredentialResolver::require_first_class(
            Some(config.dashscope()),
            "liteflow.agent.dashscope",
        )?;
        Ok(self.build_model(credential))
    }

    fn build_model(&self, credential: &PlatformCredential) -> Arc<dyn Model> {
        let mut builder = DashScopeChatModel::builder()
            .api_key(credential.api_key().unwrap_or_default())
            .model_name(&self.model_name);
        if let Some(options) = self.generate_options() {
            builder = builder.default_options(options);
        }
        if let Some(stream) = self.common.get_stream() {
            builder = builder.stream(stream);
        }
        if self.thinking_budget.is_some() {
            builder = builder.enable_thinking(true);
        }
        Arc::new(builder.build())
    }

    fn generate_options(&self) -> Option<GenerateOptions> {
        let mut options = match (self.common.generate_options(), self.thinking_budget) {
            (None, None) => return None,
            (Some(options), _) => options,
            (None, Some(_)) => GenerateOptions::default(),
        };
        options.thinking_budget = self.thinking_budget;
        Some(options)
    }
}

#[cfg(test)]
mod tests {
    use liteflow_agent_core::{AgentConfig, PlatformCredential};

    use super::DashScopeSpec;

    #[test]
    fn thinking_resolution_builds_model_without_network_io() {
        let spec = DashScopeSpec::new("qwen-test")
            .thinking(|thinking| {
                thinking.budget(4_096);
            })
            .cache_control(true)
            .stream(false);
        assert_eq!(spec.thinking_budget(), Some(4_096));

        let mut config = AgentConfig::default();
        let mut credential = PlatformCredential::default();
        credential.set_api_key(Some("test-key".to_string()));
        config.set_dashscope(credential);
        let model = spec.resolve(&config).expect("构造模型不应触网");
        assert_eq!(model.name(), "qwen-test");
    }
}
