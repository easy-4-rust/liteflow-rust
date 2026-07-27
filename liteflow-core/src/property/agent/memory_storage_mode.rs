use serde::{Deserialize, Serialize};

/// ReAct Agent 跨多次执行使用的记忆持久化后端。
///
/// 对应 Java: `com.yomahub.liteflow.property.agent.MemoryStorageMode`。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MemoryStorageMode {
    /// 完全无状态，不持久化对话历史。
    None,
    /// 仅进程内缓存，进程重启后丢失；为默认值。
    #[default]
    Jvm,
    /// 通过 AgentScope JSON Session 持久化到本地文件。
    LocalFile,
    /// 通过 AgentScope Redis Session 持久化。
    Redis,
    /// 通过 AgentScope MySQL Session 持久化。
    Mysql,
}

impl MemoryStorageMode {
    /// 兼容早期 Rust API 的进程内模式名称。
    #[allow(non_upper_case_globals)]
    #[deprecated(note = "请使用 MemoryStorageMode::Jvm")]
    pub const InMemory: Self = Self::Jvm;

    /// 兼容早期 Rust API 的自定义 Session 模式；显式 Session 仍由构建器注入。
    #[allow(non_upper_case_globals)]
    #[deprecated(note = "请使用 MemoryStorageMode::Jvm 并显式注入 AgentScope Session")]
    pub const Custom: Self = Self::Jvm;
}
