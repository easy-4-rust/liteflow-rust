use agentscope_core::model::gemini::GeminiChatModel;

/// 直接使用 API Key 构造 AgentScope Gemini 模型的工厂。
///
/// 对应 Java: `com.yomahub.liteflow.agent.gemini.GeminiModelFactory`。
pub struct GeminiModelFactory;

impl GeminiModelFactory {
    /// 构造 Gemini 模型。
    ///
    /// 对应 Java: `GeminiModelFactory#of`。
    #[must_use]
    pub fn of(api_key: impl Into<String>, model_name: impl Into<String>) -> GeminiChatModel {
        GeminiChatModel::builder()
            .api_key(api_key)
            .model_name(model_name)
            .build()
    }
}
