//! 对应 Java 类：com.yomahub.liteflow.exception.SwitchTargetCannotBePreOrFinallyException
//!
//! SWITCH 的目标节点不能为 pre/finally 类型

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 SwitchTargetCannotBePreOrFinallyException：SWITCH 的目标节点不能为 pre/finally 类型
#[derive(Debug, Clone)]
pub struct SwitchTargetCannotBePreOrFinallyException {
    /// 异常信息
    pub message: String,
}

impl SwitchTargetCannotBePreOrFinallyException {
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
    /// `SwitchTargetCannotBePreOrFinallyException#getMessage`。
    #[must_use]
    pub fn get_message(&self) -> &str {
        &self.message
    }

    /// 修改异常信息。
    ///
    /// 参数 `message` 对应 Java 同名参数。对应 Java:
    /// `SwitchTargetCannotBePreOrFinallyException#setMessage`。
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}

impl fmt::Display for SwitchTargetCannotBePreOrFinallyException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SwitchTargetCannotBePreOrFinallyException {}

impl From<SwitchTargetCannotBePreOrFinallyException> for LiteflowError {
    fn from(e: SwitchTargetCannotBePreOrFinallyException) -> Self {
        LiteflowError::TargetCannotBePreOrFinally(e.message)
    }
}
