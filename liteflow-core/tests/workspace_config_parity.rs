//! Java `WorkspaceConfig` getter 与 serde 工作区限制语义回归测试。

use liteflow_core::property::agent::WorkspaceConfig;

#[test]
fn java_named_getters_read_the_limits_used_by_workspace_tools() {
    let mut config = WorkspaceConfig::default();
    config.set_root(Some("/tmp/liteflow-agent-workspace".to_string()));
    config.set_max_file_bytes(4096);
    config.set_max_list_size(12);

    assert_eq!(config.get_root(), Some("/tmp/liteflow-agent-workspace"));
    assert_eq!(config.get_max_file_bytes(), 4096);
    assert_eq!(config.get_max_list_size(), 12);

    let value = serde_json::to_value(config).expect("WorkspaceConfig 应可序列化");
    assert_eq!(value["maxFileBytes"], 4096);
    assert_eq!(value["maxListSize"], 12);
    assert!(value.get("max_file_bytes").is_none());
}
