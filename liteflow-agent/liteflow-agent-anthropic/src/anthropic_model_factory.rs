use agentscope_core::model::anthropic::AnthropicChatModel;

/// 直接使用 API Key 构造 AgentScope Anthropic 模型的工厂。
///
/// 对应 Java: `com.yomahub.liteflow.agent.anthropic.AnthropicModelFactory`。
pub struct AnthropicModelFactory;

impl AnthropicModelFactory {
    /// 构造使用 Anthropic 官方默认地址的模型。
    ///
    /// 对应 Java: `AnthropicModelFactory#of`。
    #[must_use]
    pub fn of(api_key: impl Into<String>, model_name: impl Into<String>) -> AnthropicChatModel {
        AnthropicChatModel::builder()
            .api_key(api_key)
            .model_name(model_name)
            .build()
    }

    /// 构造使用自定义兼容 base URL 的模型。
    ///
    /// 对应 Java: `AnthropicModelFactory#custom`。
    #[must_use]
    pub fn custom(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        model_name: impl Into<String>,
    ) -> AnthropicChatModel {
        AnthropicChatModel::builder()
            .api_key(api_key)
            .base_url(base_url)
            .model_name(model_name)
            .build()
    }
}
