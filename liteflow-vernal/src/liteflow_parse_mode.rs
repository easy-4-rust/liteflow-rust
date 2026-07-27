//! 对应 Java: `com.yomahub.liteflow.enums.ParseModeEnum`。

use serde::{Deserialize, Serialize};

/// 规则解析时机。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LiteflowParseMode {
    /// 应用启动时完成规则解析，对应 Java `PARSE_ALL_ON_START`。
    #[default]
    ParseAllOnStart,
    /// 第一次执行任意链路时解析全部规则，对应 Java `PARSE_ALL_ON_FIRST_EXEC`。
    ParseAllOnFirstExec,
    /// 第一次执行相关链路时只解析该链依赖闭包。
    ParseOneOnFirstExec,
}
