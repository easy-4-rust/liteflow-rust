//! Chain 执行模式。
//!
//! 对应 Java: `com.yomahub.liteflow.enums.ChainExecuteModeEnum`。

/// 区分执行 Chain 的主体 EL 与决策路由 EL。
///
/// 对应 Java `ChainExecuteModeEnum`：
/// - `BODY` 执行 Chain 本体；
/// - `ROUTE` 执行 Chain 的决策路由。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChainExecuteModeEnum {
    /// 执行 Chain 本体 EL。
    Body,
    /// 执行 Chain 决策路由 EL。
    Route,
}
