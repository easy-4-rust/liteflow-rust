use super::OpenAiCompatibleSpec;

/// Kimi/Moonshot OpenAI 兼容平台入口。
///
/// 对应 Java: `com.yomahub.liteflow.agent.openai.Kimi`。
pub struct Kimi;

impl Kimi {
    const CONFIG_KEY: &'static str = "kimi";
    const BASE_URL: &'static str = "https://api.moonshot.cn/v1";

    /// 使用模型名称创建 Kimi 描述符。
    ///
    /// 对应 Java: `Kimi#of`。
    #[must_use]
    pub fn of(model_name: impl Into<String>) -> OpenAiCompatibleSpec {
        OpenAiCompatibleSpec::new(Self::CONFIG_KEY, model_name, Some(Self::BASE_URL))
    }
}
