//! Java `AgentConfig` getter、setter 与 serde 配置绑定语义测试。

use std::collections::HashMap;
use std::time::Duration;

use liteflow_core::property::agent::{
    AgentConfig, DefaultsConfig, LoggingConfig, PlatformCredential, SessionConfig, ShellConfig,
    SkillsConfig, WorkspaceConfig,
};
use serde_json::json;

#[test]
fn java_named_getters_read_the_same_state_written_by_setters() {
    let mut config = AgentConfig::default();

    let mut workspace = WorkspaceConfig::default();
    workspace.set_root(Some("/tmp/liteflow-agent".to_string()));
    config.set_workspace(workspace.clone());

    let mut session = SessionConfig::default();
    session.set_max_sessions(128);
    config.set_session(session.clone());

    let mut shell = ShellConfig::default();
    shell.set_timeout(Duration::from_secs(17));
    config.set_shell(shell.clone());

    let mut defaults = DefaultsConfig::default();
    defaults.set_max_iterations(23);
    config.set_defaults(defaults.clone());

    let mut logging = LoggingConfig::default();
    logging.set_react_enabled(false);
    config.set_logging(logging.clone());

    let mut skills = SkillsConfig::default();
    skills.set_enabled(true);
    skills.set_path("./agent-skills".to_string());
    config.set_skills(skills.clone());

    let credential = |api_key: &str| PlatformCredential {
        api_key: Some(api_key.to_string()),
        ..PlatformCredential::default()
    };
    let openai = credential("openai-key");
    let anthropic = credential("anthropic-key");
    let gemini = credential("gemini-key");
    let dashscope = credential("dashscope-key");
    config.set_openai(openai.clone());
    config.set_anthropic(anthropic.clone());
    config.set_gemini(gemini.clone());
    config.set_dashscope(dashscope.clone());

    let openai_compatible = HashMap::from([("deepseek".to_string(), credential("deepseek-key"))]);
    let anthropic_compatible = HashMap::from([("private".to_string(), credential("private-key"))]);
    config.set_openai_compatible(openai_compatible.clone());
    config.set_anthropic_compatible(anthropic_compatible.clone());

    assert_eq!(config.get_workspace(), &workspace);
    assert_eq!(config.get_session(), &session);
    assert_eq!(config.get_shell(), &shell);
    assert_eq!(config.get_defaults(), &defaults);
    assert_eq!(config.get_logging(), &logging);
    assert_eq!(config.get_skills(), &skills);
    assert_eq!(config.get_openai(), &openai);
    assert_eq!(config.get_anthropic(), &anthropic);
    assert_eq!(config.get_gemini(), &gemini);
    assert_eq!(config.get_dashscope(), &dashscope);
    assert_eq!(config.get_openai_compatible(), &openai_compatible);
    assert_eq!(config.get_anthropic_compatible(), &anthropic_compatible);
}

#[test]
fn jackson_camel_case_shape_round_trips_through_serde() {
    let mut config = AgentConfig::default();
    config.set_openai_compatible(HashMap::from([(
        "deepseek".to_string(),
        PlatformCredential {
            api_key: Some("secret".to_string()),
            base_url: Some("https://example.invalid/v1".to_string()),
            ..PlatformCredential::default()
        },
    )]));

    let value = serde_json::to_value(&config).expect("AgentConfig 应可序列化");
    assert_eq!(
        value["openaiCompatible"]["deepseek"]["baseUrl"],
        json!("https://example.invalid/v1")
    );
    assert!(value.get("openai_compatible").is_none());

    let decoded: AgentConfig =
        serde_json::from_value(value).expect("camelCase AgentConfig 应可反序列化");
    assert_eq!(decoded, config);
}
