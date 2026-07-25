//! 对应 Java 类：com.yomahub.liteflow.exception.AndOrConditionException

use crate::exception::lite_flow_exception::LiteFlowException;

/// AND/OR 条件异常。
#[derive(Debug, Clone)]
pub struct AndOrConditionException {
    message: String,
}

impl AndOrConditionException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for AndOrConditionException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AndOrConditionException {}

impl LiteFlowException for AndOrConditionException {
    fn message(&self) -> &str {
        &self.message
    }
}
