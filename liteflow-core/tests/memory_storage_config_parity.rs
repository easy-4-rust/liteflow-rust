//! Java `MemoryStorageConfig` 访问器及 serde 配置绑定语义回归测试。

use liteflow_core::property::agent::{MemoryStorageConfig, MemoryStorageMode};

#[test]
fn java_named_getters_read_the_configuration_used_by_agent_sessions() {
    let mut config = MemoryStorageConfig::default();
    assert_eq!(config.get_mode(), MemoryStorageMode::Jvm);

    config.set_mode(MemoryStorageMode::Redis);
    assert_eq!(config.get_mode(), MemoryStorageMode::Redis);
    assert_eq!(config.get_local_file(), config.local_file());
    assert_eq!(config.get_redis(), config.redis());
    assert_eq!(config.get_mysql(), config.mysql());
}

#[test]
fn jackson_camel_case_shape_round_trips_through_serde() {
    let value = serde_json::json!({
        "mode": "REDIS",
        "loadOnFirstUse": false,
        "saveAfterCall": true,
        "saveOnError": false
    });
    let config: MemoryStorageConfig =
        serde_json::from_value(value).expect("camelCase MemoryStorageConfig 应可反序列化");

    assert_eq!(config.get_mode(), MemoryStorageMode::Redis);
    assert!(!config.is_load_on_first_use());
    assert!(config.is_save_after_call());
    assert!(!config.is_save_on_error());

    let encoded = serde_json::to_value(config).expect("MemoryStorageConfig 应可序列化");
    assert!(encoded.get("localFile").is_some());
    assert!(encoded.get("local_file").is_none());
}
