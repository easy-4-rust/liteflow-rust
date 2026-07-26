//! 可执行对象类型。
//!
//! 对应 Java: `com.yomahub.liteflow.enums.ExecuteableTypeEnum`。
//! Java 原类型名保留了 `Executeable` 拼写，Rust 端也保持对象名一致。

/// 标识统一可执行对象是 Chain、Condition 还是 Node。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecuteableTypeEnum {
    /// 链路级可执行对象。
    Chain,
    /// 条件级可执行对象。
    Condition,
    /// 节点级可执行对象。
    Node,
}
