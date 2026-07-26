//! Agent 模型提供商聚合 crate 的无网络构建契约测试。

#[cfg(feature = "anthropic")]
use liteflow_agent::AnthropicAgentModelConfig;
#[cfg(feature = "dashscope")]
use liteflow_agent::DashScopeAgentModelConfig;
#[cfg(feature = "gemini")]
use liteflow_agent::GeminiAgentModelConfig;
#[cfg(feature = "openai")]
use liteflow_agent::OpenAiAgentModelConfig;

#[test]
fn enabled_provider_configs_build_real_agentscope_models_without_network_io() {
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
}
