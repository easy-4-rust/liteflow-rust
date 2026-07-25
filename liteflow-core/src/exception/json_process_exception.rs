//! 对应 Java 类：com.yomahub.liteflow.exception.JsonProcessException

use crate::exception::lite_flow_exception::LiteFlowException;

/// JSON 处理异常。
#[derive(Debug, Clone)]
pub struct JsonProcessException {
    message: String,
    cause: Option<String>,
}

impl JsonProcessException {
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
}

impl std::fmt::Display for JsonProcessException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for JsonProcessException {}

impl LiteFlowException for JsonProcessException {
    fn message(&self) -> &str {
        &self.message
    }
}
