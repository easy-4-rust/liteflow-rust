use serde::{Deserialize, Serialize};

/// Shell 工具的命令过滤模式。
///
/// 对应 Java: `com.yomahub.liteflow.property.agent.ShellMode`。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ShellMode {
    /// 仅允许白名单中的命令，为默认模式。
    #[default]
    Whitelist,
    /// 允许黑名单之外的命令。
    Blacklist,
    /// 拒绝全部 Shell 命令。
    Disabled,
}
