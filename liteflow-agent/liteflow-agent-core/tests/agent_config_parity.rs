//! Java `property.agent` 配置树的 serde、默认值和方法契约测试。

use std::time::Duration;

use liteflow_agent_core::{
    AgentConfig, DefaultsConfig, LocalFileMemoryConfig, MemoryStorageMode, RedisClientType,
    ShellMode,
};
use serde_json::json;

#[test]
fn java_agent_config_defaults_and_setters_are_preserved() {
    let mut config = AgentConfig::default();
    assert_eq!(config.defaults().max_iterations(), 50);
    assert_eq!(config.session().idle_timeout(), Duration::from_secs(1_800));
    assert_eq!(config.session().cleanup_interval(), Duration::from_secs(60));
    assert_eq!(config.session().max_sessions(), 10_000);
    assert_eq!(config.session().memory().mode(), MemoryStorageMode::Jvm);
    assert!(config.session().memory().is_load_on_first_use());
    assert!(config.session().memory().is_save_after_call());
    assert!(config.session().memory().is_save_on_error());
    assert_eq!(config.shell().mode(), ShellMode::Whitelist);
    assert_eq!(config.shell().timeout(), Duration::from_secs(30));
    assert_eq!(config.shell().max_output_bytes(), 1024 * 1024);
    assert!(config.shell().whitelist().iter().any(|item| item == "jq"));
    assert!(config.shell().blacklist().iter().any(|item| item == "rm"));
    assert!(config.logging().is_react_enabled());
    assert!(!config.skills().is_enabled());
    assert_eq!(config.skills().path(), "./skills");
    assert!(config.skills().is_strict());
    assert_eq!(config.workspace().max_file_bytes(), 10 * 1024 * 1024);
    assert_eq!(config.workspace().max_list_size(), 1_000);
    assert_eq!(LocalFileMemoryConfig::SUB_DIR, ".agent-session");

    let mut defaults = DefaultsConfig::default();
    defaults.set_max_iterations(23);
    config.set_defaults(defaults);
    assert_eq!(config.defaults().max_iterations(), 23);
}

#[test]
fn java_agent_config_deserializes_nested_camel_case_and_human_durations() {
    let config: AgentConfig = serde_json::from_value(json!({
        "workspace": {
            "root": "/tmp/liteflow-agent",
            "autoCreate": false,
            "cleanupOnSessionExpire": false,
            "cleanupOnJvmShutdown": true,
            "maxFileBytes": 4096,
            "maxListSize": 20
        },
        "session": {
            "idleTimeout": "45m",
            "cleanupInterval": "2m",
            "maxSessions": 12,
            "memory": {
                "mode": "REDIS",
                "loadOnFirstUse": false,
                "saveAfterCall": false,
                "saveOnError": false,
                "redis": {
                    "beanName": "redisClient",
                    "clientType": "LETTUCE",
                    "keyPrefix": "demo:session"
                }
            }
        },
        "shell": {
            "mode": "BLACKLIST",
            "timeout": "5s",
            "maxOutputBytes": 2048
        },
        "defaults": {"maxIterations": 77},
        "logging": {"reactEnabled": false},
        "skills": {"enabled": true, "path": "./custom-skills", "strict": false},
        "openai": {
            "apiKey": "secret",
            "baseUrl": "https://gateway.example",
            "extra": {"tenant": "demo"}
        }
    }))
    .expect("完整 Agent 配置树应由 serde 解析");

    assert_eq!(config.workspace().root(), Some("/tmp/liteflow-agent"));
    assert!(!config.workspace().is_auto_create());
    assert_eq!(config.session().idle_timeout(), Duration::from_secs(2_700));
    assert_eq!(
        config.session().cleanup_interval(),
        Duration::from_secs(120)
    );
    assert_eq!(config.session().memory().mode(), MemoryStorageMode::Redis);
    assert_eq!(
        config.session().memory().redis().client_type(),
        RedisClientType::Lettuce
    );
    assert_eq!(
        config.session().memory().redis().bean_name(),
        Some("redisClient")
    );
    assert_eq!(config.shell().mode(), ShellMode::Blacklist);
    assert_eq!(config.shell().timeout(), Duration::from_secs(5));
    assert_eq!(config.defaults().max_iterations(), 77);
    assert!(!config.logging().is_react_enabled());
    assert!(config.skills().is_enabled());
    assert_eq!(config.openai().api_key(), Some("secret"));
    assert_eq!(
        config.openai().extra().get("tenant").map(String::as_str),
        Some("demo")
    );

    let serialized = serde_json::to_value(&config).expect("配置应可序列化");
    assert_eq!(serialized["session"]["memory"]["mode"], "REDIS");
    assert_eq!(serialized["shell"]["mode"], "BLACKLIST");
}
