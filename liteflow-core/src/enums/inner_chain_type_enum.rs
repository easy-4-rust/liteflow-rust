//! 匿名或内部链路的执行环境标记。

/// 标识 Chain 是否处于串行或并行的隐式执行环境。
///
/// 对应 Java: `com.yomahub.liteflow.enums.InnerChainTypeEnum`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InnerChainTypeEnum {
    /// 不是隐式 Chain。
    None,
    /// 在串行环境中执行。
    InSync,
    /// 在并行环境中执行。
    InAsync,
}
