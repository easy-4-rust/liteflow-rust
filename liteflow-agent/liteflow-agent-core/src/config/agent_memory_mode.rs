use super::MemoryStorageMode;

/// 早期 Rust API 的记忆模式名称。
///
/// 新代码应使用与 Java 对象一一对应的 `MemoryStorageMode`；该别名仅保留源码兼容。
#[deprecated(note = "请使用 MemoryStorageMode")]
pub type AgentMemoryMode = MemoryStorageMode;
