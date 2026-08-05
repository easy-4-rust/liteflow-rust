use super::OpenAiCompatibleSpec;

/// DeepSeek OpenAI 兼容平台入口。
///
/// 对应 Java: `com.yomahub.liteflow.agent.openai.DeepSeek`。
pub struct DeepSeek;

impl DeepSeek {
    const CONFIG_KEY: &'static str = "deepseek";
    const BASE_URL: &'static str = "https://api.deepseek.com/v1";

    /// 使用模型名称创建 DeepSeek 描述符。
    ///
    /// 对应 Java: `DeepSeek#of`。
    #[must_use]
    pub fn of(model_name: impl Into<String>) -> OpenAiCompatibleSpec {
        OpenAiCompatibleSpec::new(Self::CONFIG_KEY, model_name, Some(Self::BASE_URL))
    }
}
