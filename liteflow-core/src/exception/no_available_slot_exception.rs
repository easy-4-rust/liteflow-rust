//! 对应 Java 类：com.yomahub.liteflow.exception.NoAvailableSlotException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 无可用槽位异常。
#[derive(Debug, Clone)]
pub struct NoAvailableSlotException {
    message: String,
}

impl NoAvailableSlotException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for NoAvailableSlotException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NoAvailableSlotException {}

impl LiteFlowException for NoAvailableSlotException {
    fn message(&self) -> &str {
        &self.message
    }
}
