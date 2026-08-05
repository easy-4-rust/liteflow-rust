use agentscope_core::model::openai::OpenAIChatModel;

use super::OpenAiModelFactory;

/// 使用显式 API Key 的常用 OpenAI 兼容平台模型预设。
///
/// 对应 Java: `com.yomahub.liteflow.agent.openai.OpenAICompatiblePresets`。
pub struct OpenAiCompatiblePresets;

impl OpenAiCompatiblePresets {
    /// 构造 DeepSeek 兼容模型。对应 Java: `OpenAICompatiblePresets#deepseek`。
    #[must_use]
    pub fn deepseek(api_key: impl Into<String>, model_name: impl Into<String>) -> OpenAIChatModel {
        OpenAiModelFactory::custom(api_key, "https://api.deepseek.com/v1", model_name)
    }

    /// 构造 Kimi 兼容模型。对应 Java: `OpenAICompatiblePresets#kimi`。
    #[must_use]
    pub fn kimi(api_key: impl Into<String>, model_name: impl Into<String>) -> OpenAIChatModel {
        OpenAiModelFactory::custom(api_key, "https://api.moonshot.cn/v1", model_name)
    }

    /// 构造 GLM 兼容模型。对应 Java: `OpenAICompatiblePresets#glm`。
    #[must_use]
    pub fn glm(api_key: impl Into<String>, model_name: impl Into<String>) -> OpenAIChatModel {
        OpenAiModelFactory::custom(api_key, "https://open.bigmodel.cn/api/paas/v4", model_name)
    }

    /// 构造 MiniMax 兼容模型。对应 Java: `OpenAICompatiblePresets#minimax`。
    #[must_use]
    pub fn minimax(api_key: impl Into<String>, model_name: impl Into<String>) -> OpenAIChatModel {
        OpenAiModelFactory::custom(api_key, "https://api.minimax.chat/v1", model_name)
    }
}
