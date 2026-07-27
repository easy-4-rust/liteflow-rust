//! Java `MysqlMemoryConfig` getter 与 serde 配置绑定语义回归测试。

use liteflow_core::property::agent::MysqlMemoryConfig;

#[test]
fn java_named_getters_read_the_backend_configuration_written_by_setters() {
    let mut config = MysqlMemoryConfig::default();
    config.set_data_source_bean_name(Some("agentDataSource".to_string()));
    config.set_database_name(Some("agent_db".to_string()));
    config.set_table_name(Some("agent_sessions".to_string()));

    assert_eq!(config.get_data_source_bean_name(), Some("agentDataSource"));
    assert_eq!(config.get_database_name(), Some("agent_db"));
    assert_eq!(config.get_table_name(), Some("agent_sessions"));

    let value = serde_json::to_value(config).expect("MysqlMemoryConfig 应可序列化");
    assert_eq!(value["dataSourceBeanName"], "agentDataSource");
    assert_eq!(value["databaseName"], "agent_db");
    assert_eq!(value["tableName"], "agent_sessions");
}
