use super::OpenAiCompatibleSpec;

/// 智谱 GLM OpenAI 兼容平台入口。
///
/// 对应 Java: `com.yomahub.liteflow.agent.openai.GLM`。
pub struct Glm;

impl Glm {
    const CONFIG_KEY: &'static str = "glm";
    const BASE_URL: &'static str = "https://open.bigmodel.cn/api/paas/v4";

    /// 使用模型名称创建 GLM 描述符。
    ///
    /// 对应 Java: `GLM#of`。
    #[must_use]
    pub fn of(model_name: impl Into<String>) -> OpenAiCompatibleSpec {
        OpenAiCompatibleSpec::new(Self::CONFIG_KEY, model_name, Some(Self::BASE_URL))
    }
}
