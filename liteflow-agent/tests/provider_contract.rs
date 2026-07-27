//! Agent 模型提供商聚合 crate 的无网络构建契约测试。
//!
//! 每个平台验证：
//! 1. 配置可序列化且字段名为 camelCase
//! 2. `build()` 产出 `.name()` 非空的 `Arc<dyn Model>`，且不触网

#[cfg(feature = "anthropic")]
use liteflow_agent::AnthropicAgentModelConfig;
#[cfg(feature = "bedrock")]
use liteflow_agent::BedrockAgentModelConfig;
#[cfg(feature = "compatible")]
use liteflow_agent::CompatibleAgentModelConfig;
#[cfg(feature = "copilot")]
use liteflow_agent::CopilotAgentModelConfig;
#[cfg(feature = "dashscope")]
use liteflow_agent::DashScopeAgentModelConfig;
#[cfg(feature = "gemini")]
use liteflow_agent::GeminiAgentModelConfig;
#[cfg(feature = "glm")]
use liteflow_agent::GlmAgentModelConfig;
#[cfg(feature = "openai")]
use liteflow_agent::OpenAiAgentModelConfig;
#[cfg(feature = "openrouter")]
use liteflow_agent::OpenRouterAgentModelConfig;
#[cfg(feature = "telnyx")]
use liteflow_agent::TelnyxAgentModelConfig;

#[test]
fn enabled_provider_configs_build_real_agentscope_models_without_network_io() {
    // ── agentscope 原生平台 ──

    #[cfg(feature = "openai")]
    {
        let mut config = OpenAiAgentModelConfig::new("test-openai-key", "gpt-test");
        config.base_url = Some("https://openai.example.test/v1".to_string());
        let json = serde_json::to_value(&config).expect("OpenAI config should serialize");
        assert_eq!(json["modelName"], "gpt-test");
        assert!(!config.build().name().is_empty());
    }

    #[cfg(feature = "anthropic")]
    {
        let config = AnthropicAgentModelConfig::new("test-anthropic-key", "claude-test");
        let json = serde_json::to_value(&config).expect("Anthropic config should serialize");
        assert_eq!(json["modelName"], "claude-test");
        assert!(!config.build().name().is_empty());
    }

    #[cfg(feature = "gemini")]
    {
        let config = GeminiAgentModelConfig::new("test-gemini-key", "gemini-test");
        let json = serde_json::to_value(&config).expect("Gemini config should serialize");
        assert_eq!(json["modelName"], "gemini-test");
        assert!(!config.build().name().is_empty());
    }

    #[cfg(feature = "dashscope")]
    {
        let config = DashScopeAgentModelConfig::new("test-dashscope-key", "qwen-test");
        let json = serde_json::to_value(&config).expect("DashScope config should serialize");
        assert_eq!(json["modelName"], "qwen-test");
        assert!(!config.build().name().is_empty());
    }

    // ── zeroclaw 衍生平台 ──

    #[cfg(feature = "telnyx")]
    {
        let config = TelnyxAgentModelConfig::new("test-telnyx-key", "openai/gpt-4o");
        let json = serde_json::to_value(&config).expect("Telnyx config should serialize");
        assert_eq!(json["modelName"], "openai/gpt-4o");
        assert!(!config.build().name().is_empty());
    }

    #[cfg(feature = "glm")]
    {
        let config = GlmAgentModelConfig::new("test-id.test-secret", "glm-4.6");
        let json = serde_json::to_value(&config).expect("GLM config should serialize");
        assert_eq!(json["modelName"], "glm-4.6");
        // GLM build 不触网：JWT 在首次请求时才生成
        assert!(!config.build().name().is_empty());
    }

    #[cfg(feature = "openrouter")]
    {
        let config = OpenRouterAgentModelConfig::new("test-or-key", "anthropic/claude-3.5-sonnet");
        let json = serde_json::to_value(&config).expect("OpenRouter config should serialize");
        assert_eq!(json["modelName"], "anthropic/claude-3.5-sonnet");
        assert!(!config.build().name().is_empty());
    }

    #[cfg(feature = "copilot")]
    {
        let config = CopilotAgentModelConfig::new(Some("ghp_test"), "gpt-4o");
        let json = serde_json::to_value(&config).expect("Copilot config should serialize");
        assert_eq!(json["modelName"], "gpt-4o");
        // Copilot build 不触网：OAuth 在首次请求时才发起
        assert!(!config.build().name().is_empty());
    }

    #[cfg(feature = "bedrock")]
    {
        let config = BedrockAgentModelConfig::new(
            "AKIATEST",
            "secrettest",
            "us-east-1",
            "anthropic.claude-3-5-sonnet-20241022-v2:0",
        );
        let json = serde_json::to_value(&config).expect("Bedrock config should serialize");
        assert_eq!(json["modelId"], "anthropic.claude-3-5-sonnet-20241022-v2:0");
        assert!(!config.build().name().is_empty());
    }

    #[cfg(feature = "compatible")]
    {
        let config = CompatibleAgentModelConfig::deepseek("test-deepseek-key", "deepseek-chat");
        let json = serde_json::to_value(&config).expect("Compatible config should serialize");
        assert_eq!(json["modelName"], "deepseek-chat");
        assert_eq!(json["baseUrl"], "https://api.deepseek.com/v1");
        assert!(!config.build().name().is_empty());
    }
}
