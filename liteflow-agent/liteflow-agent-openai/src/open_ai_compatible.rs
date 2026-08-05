use super::OpenAiCompatibleSpec;

/// 自定义 OpenAI 兼容厂商的兜底入口。
///
/// 用户需在 `liteflow.agent.openai-compatible.<config_key>` 下配置 API Key；该入口
/// 不提供默认 base URL。
///
/// 对应 Java: `com.yomahub.liteflow.agent.openai.OpenAICompatible`。
pub struct OpenAiCompatible;

impl OpenAiCompatible {
    /// 创建自定义兼容平台描述符。
    ///
    /// 对应 Java: `OpenAICompatible#custom`。
    #[must_use]
    pub fn custom(
        config_key: impl Into<String>,
        model_name: impl Into<String>,
    ) -> OpenAiCompatibleSpec {
        OpenAiCompatibleSpec::new(config_key, model_name, None::<String>)
    }
}
