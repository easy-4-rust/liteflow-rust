//! 对应 Java 类：com.yomahub.liteflow.exception.NoAvailableSlotException
//!
//! 无可用 Slot（执行上下文未分配）

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 NoAvailableSlotException：无可用 Slot（执行上下文未分配）
#[derive(Debug, Clone)]
pub struct NoAvailableSlotException {
    /// 异常信息
    pub message: String,
}

impl NoAvailableSlotException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for NoAvailableSlotException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NoAvailableSlotException {}

impl From<NoAvailableSlotException> for LiteflowError {
    fn from(e: NoAvailableSlotException) -> Self {
        LiteflowError::NoAvailableSlot(e.message)
    }
}
