use super::DashScopeSpec;

/// DashScope 官方 API 模型描述符入口。
///
/// 对应 Java: `com.yomahub.liteflow.agent.dashscope.DashScope`。
pub struct DashScope;

impl DashScope {
    /// 使用模型名称创建 DashScope 描述符。
    ///
    /// 对应 Java: `DashScope#of`。
    #[must_use]
    pub fn of(model_name: impl Into<String>) -> DashScopeSpec {
        DashScopeSpec::new(model_name)
    }
}
