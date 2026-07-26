//! 对应 Java: `com.yomahub.liteflow.enums.ParseModeEnum`。

use serde::{Deserialize, Serialize};

/// 规则解析时机。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LiteflowParseMode {
    /// 应用启动时完成规则解析，对应 Java `PARSE_ALL_ON_START`。
    #[default]
    ParseAllOnStart,
    /// 首次执行时解析；当前桥接层保留配置值并在执行入口前保证初始化。
    ParseOneOnFirstExec,
}
