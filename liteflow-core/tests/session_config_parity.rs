//! Java `SessionConfig` 访问器及会话管理器配置语义回归测试。

use std::time::Duration;

use liteflow_core::property::agent::{MemoryStorageConfig, MemoryStorageMode, SessionConfig};

#[test]
fn java_named_getters_share_state_with_setters_and_agent_consumers() {
    let mut memory = MemoryStorageConfig::default();
    memory.set_mode(MemoryStorageMode::Mysql);

    let mut config = SessionConfig::default();
    config.set_idle_timeout(Duration::from_secs(90));
    config.set_cleanup_interval(Duration::from_secs(7));
    config.set_max_sessions(32);
    config.set_memory(memory);

    assert_eq!(config.get_idle_timeout(), Duration::from_secs(90));
    assert_eq!(config.get_cleanup_interval(), Duration::from_secs(7));
    assert_eq!(config.get_max_sessions(), 32);
    assert_eq!(config.get_memory().get_mode(), MemoryStorageMode::Mysql);
}
