use serde::{Deserialize, Serialize};

/// ReAct Agent 日志开关配置，对应配置段 `liteflow.agent.logging.*`。
///
/// 对应 Java: `com.yomahub.liteflow.property.agent.LoggingConfig`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LoggingConfig {
    /// 是否输出 reason、act、error 等 ReAct 内部事件，默认开启。
    pub react_enabled: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            react_enabled: true,
        }
    }
}

impl LoggingConfig {
    /// 返回是否启用 ReAct 内部事件日志。对应 Java: `LoggingConfig#isReactEnabled`。
    #[must_use]
    pub fn is_react_enabled(&self) -> bool {
        self.react_enabled
    }

    /// 设置 ReAct 内部事件日志开关。对应 Java: `LoggingConfig#setReactEnabled`。
    pub fn set_react_enabled(&mut self, react_enabled: bool) {
        self.react_enabled = react_enabled;
    }
}
