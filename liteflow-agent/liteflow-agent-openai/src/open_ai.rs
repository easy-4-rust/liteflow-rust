use super::OpenAiSpec;

/// OpenAI 官方 API 的模型描述符入口。
///
/// 凭证来源为 `liteflow.agent.openai`。
///
/// 对应 Java: `com.yomahub.liteflow.agent.openai.OpenAI`。
pub struct OpenAi;

impl OpenAi {
    /// 使用模型名称创建 OpenAI 描述符。
    ///
    /// 对应 Java: `OpenAI#of`。
    #[must_use]
    pub fn of(model_name: impl Into<String>) -> OpenAiSpec {
        OpenAiSpec::new(model_name)
    }
}
