use agentscope_core::model::dashscope::DashScopeChatModel;

/// 直接使用 API Key 构造 AgentScope DashScope 模型的工厂。
///
/// 对应 Java: `com.yomahub.liteflow.agent.dashscope.DashScopeModelFactory`。
pub struct DashScopeModelFactory;

impl DashScopeModelFactory {
    /// 构造 DashScope 模型。
    ///
    /// 对应 Java: `DashScopeModelFactory#of`。
    #[must_use]
    pub fn of(api_key: impl Into<String>, model_name: impl Into<String>) -> DashScopeChatModel {
        DashScopeChatModel::builder()
            .api_key(api_key)
            .model_name(model_name)
            .build()
    }
}
