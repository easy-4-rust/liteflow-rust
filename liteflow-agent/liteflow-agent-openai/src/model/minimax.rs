use super::OpenAiCompatibleSpec;

/// MiniMax OpenAI 兼容平台入口。
///
/// 对应 Java: `com.yomahub.liteflow.agent.openai.Minimax`。
pub struct Minimax;

impl Minimax {
    const CONFIG_KEY: &'static str = "minimax";
    const BASE_URL: &'static str = "https://api.minimax.chat/v1";

    /// 使用模型名称创建 MiniMax 描述符。
    ///
    /// 对应 Java: `Minimax#of`。
    #[must_use]
    pub fn of(model_name: impl Into<String>) -> OpenAiCompatibleSpec {
        OpenAiCompatibleSpec::new(Self::CONFIG_KEY, model_name, Some(Self::BASE_URL))
    }
}
