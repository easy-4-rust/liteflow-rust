use super::AnthropicSpec;

/// Anthropic 官方 API 模型描述符入口。
///
/// 对应 Java: `com.yomahub.liteflow.agent.anthropic.Anthropic`。
pub struct Anthropic;

impl Anthropic {
    /// 使用模型名称创建 Anthropic 描述符。
    ///
    /// 对应 Java: `Anthropic#of`。
    #[must_use]
    pub fn of(model_name: impl Into<String>) -> AnthropicSpec {
        AnthropicSpec::new(model_name)
    }
}
