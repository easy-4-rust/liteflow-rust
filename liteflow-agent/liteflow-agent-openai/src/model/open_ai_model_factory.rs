use agentscope_core::model::openai::OpenAIChatModel;

/// 直接使用 API Key 构造 AgentScope OpenAI 模型的工厂。
///
/// 对应 Java: `com.yomahub.liteflow.agent.openai.OpenAIModelFactory`。
pub struct OpenAiModelFactory;

impl OpenAiModelFactory {
    /// 构造使用 OpenAI 官方默认地址的模型。
    ///
    /// 对应 Java: `OpenAIModelFactory#openai`。
    #[must_use]
    pub fn openai(api_key: impl Into<String>, model_name: impl Into<String>) -> OpenAIChatModel {
        OpenAIChatModel::builder()
            .api_key(api_key)
            .model_name(model_name)
            .build()
    }

    /// 构造使用自定义兼容 base URL 的模型。
    ///
    /// 对应 Java: `OpenAIModelFactory#custom`。
    #[must_use]
    pub fn custom(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        model_name: impl Into<String>,
    ) -> OpenAIChatModel {
        OpenAIChatModel::builder()
            .api_key(api_key)
            .base_url(base_url)
            .model_name(model_name)
            .build()
    }
}
