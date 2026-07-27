use super::AnthropicSpec;

/// 自定义 Anthropic 兼容平台描述符入口。
///
/// 对应 Java: `com.yomahub.liteflow.agent.anthropic.AnthropicCompatible`。
pub struct AnthropicCompatible;

impl AnthropicCompatible {
    /// 使用兼容配置 key 与模型名称创建描述符。
    ///
    /// 对应 Java: `AnthropicCompatible#custom`。
    #[must_use]
    pub fn custom(config_key: impl Into<String>, model_name: impl Into<String>) -> AnthropicSpec {
        AnthropicSpec::with_compatible_key(model_name, Some(config_key))
    }
}
