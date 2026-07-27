//! Java `RedisMemoryConfig` getter 与 serde 配置绑定语义回归测试。

use liteflow_core::property::agent::{RedisClientType, RedisMemoryConfig};

#[test]
fn java_named_getters_read_the_backend_configuration_written_by_setters() {
    let mut config = RedisMemoryConfig::default();
    config.set_bean_name(Some("agentRedis".to_string()));
    config.set_client_type(RedisClientType::Lettuce);
    config.set_key_prefix("orders:agent");

    assert_eq!(config.get_bean_name(), Some("agentRedis"));
    assert_eq!(config.get_client_type(), RedisClientType::Lettuce);
    assert_eq!(config.get_key_prefix(), "orders:agent");

    let value = serde_json::to_value(config).expect("RedisMemoryConfig 应可序列化");
    assert_eq!(value["beanName"], "agentRedis");
    assert_eq!(value["clientType"], "LETTUCE");
    assert_eq!(value["keyPrefix"], "orders:agent");
}
