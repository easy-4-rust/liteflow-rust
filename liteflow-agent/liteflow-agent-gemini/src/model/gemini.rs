use super::GeminiSpec;

/// Gemini 官方 API 模型描述符入口。
///
/// 对应 Java: `com.yomahub.liteflow.agent.gemini.Gemini`。
pub struct Gemini;

impl Gemini {
    /// 使用模型名称创建 Gemini 描述符。
    ///
    /// 对应 Java: `Gemini#of`。
    #[must_use]
    pub fn of(model_name: impl Into<String>) -> GeminiSpec {
        GeminiSpec::new(model_name)
    }
}
