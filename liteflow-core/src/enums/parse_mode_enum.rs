//! 规则解析模式。
//!
//! 对应 Java: `com.yomahub.liteflow.enums.ParseModeEnum`。

/// 控制规则是在启动阶段还是首次执行阶段解析。
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ParseModeEnum {
    /// 启动时解析全部规则。
    #[default]
    ParseAllOnStart,
    /// 第一次执行任意链路时解析全部规则。
    ParseAllOnFirstExec,
    /// 第一次执行相关链路时只解析当前规则。
    ParseOneOnFirstExec,
}
