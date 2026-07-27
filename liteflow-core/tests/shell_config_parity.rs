//! Java `ShellConfig` 安全字段访问和 serde 语义回归测试。

use std::time::Duration;

use liteflow_core::property::agent::{ShellConfig, ShellMode};

#[test]
fn java_named_accessors_update_the_configuration_consumed_by_shell_tool() {
    let mut config = ShellConfig::default();

    assert_eq!(config.get_mode(), ShellMode::Whitelist);
    assert!(config.get_whitelist().iter().any(|command| command == "ls"));
    assert!(config.get_blacklist().iter().any(|command| command == "rm"));
    assert_eq!(config.get_timeout(), Duration::from_secs(30));
    assert_eq!(config.get_max_output_bytes(), 1024 * 1024);

    config.set_mode(ShellMode::Blacklist);
    config.set_whitelist(vec!["jq".to_string()]);
    config.set_blacklist(vec!["shutdown".to_string()]);
    config.set_timeout(Duration::from_secs(7));
    config.set_max_output_bytes(4096);

    assert_eq!(config.get_mode(), ShellMode::Blacklist);
    assert_eq!(config.get_whitelist(), ["jq".to_string()]);
    assert_eq!(config.get_blacklist(), ["shutdown".to_string()]);
    assert_eq!(config.get_timeout(), Duration::from_secs(7));
    assert_eq!(config.get_max_output_bytes(), 4096);
}
