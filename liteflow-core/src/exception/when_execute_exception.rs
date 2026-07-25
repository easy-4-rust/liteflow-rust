//! 对应 Java 类：com.yomahub.liteflow.exception.WhenExecuteException

use crate::exception::lite_flow_exception::LiteFlowException;

/// WHEN 执行异常。
#[derive(Debug, Clone)]
pub struct WhenExecuteException {
    message: String,
    cause: Option<String>,
}

impl WhenExecuteException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            cause: None,
        }
    }

    pub fn with_cause(message: impl Into<String>, cause: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            cause: Some(cause.into()),
        }
    }

    pub fn cause_message(&self) -> Option<&str> {
        self.cause.as_deref()
    }
}

impl std::fmt::Display for WhenExecuteException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for WhenExecuteException {}

impl LiteFlowException for WhenExecuteException {
    fn message(&self) -> &str {
        &self.message
    }
}
