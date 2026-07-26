//! LiteFlow Agent 配置集成场景。

use liteflow_agent_core::{AgentConfig, MemoryStorageMode};

/// 校验 Agent 默认配置与内存会话模式。
pub async fn run_case() -> bool {
    let config = AgentConfig::default();
    config.defaults.max_iterations > 0
        && config.publish_events
        && config.session.memory.mode == MemoryStorageMode::Jvm
}
