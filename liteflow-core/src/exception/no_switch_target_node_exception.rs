//! 对应 Java 类：com.yomahub.liteflow.exception.NoSwitchTargetNodeException
//!
//! SWITCH 条件无匹配目标节点

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 NoSwitchTargetNodeException：SWITCH 条件无匹配目标节点
#[derive(Debug, Clone)]
pub struct NoSwitchTargetNodeException {
    /// 异常信息
    pub message: String,
}

impl NoSwitchTargetNodeException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// 返回异常信息。
    ///
    /// # 返回
    /// 当前异常持有的原始消息。对应 Java:
    /// `NoSwitchTargetNodeException#getMessage`。
    #[must_use]
    pub fn get_message(&self) -> &str {
        &self.message
    }

    /// 修改异常信息。
    ///
    /// 参数 `message` 对应 Java 同名参数。对应 Java:
    /// `NoSwitchTargetNodeException#setMessage`。
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}

impl fmt::Display for NoSwitchTargetNodeException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NoSwitchTargetNodeException {}

impl From<NoSwitchTargetNodeException> for LiteflowError {
    fn from(e: NoSwitchTargetNodeException) -> Self {
        LiteflowError::NoSwitchTarget(e.message)
    }
}
