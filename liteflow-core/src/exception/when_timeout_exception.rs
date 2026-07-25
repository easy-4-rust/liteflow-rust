//! 对应 Java 类：com.yomahub.liteflow.exception.WhenTimeoutException

use crate::exception::lite_flow_exception::LiteFlowException;

/// WHEN 超时异常。
#[derive(Debug, Clone)]
pub struct WhenTimeoutException {
    message: String,
    timeout_ms: Option<u64>,
}

impl WhenTimeoutException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            timeout_ms: None,
        }
    }

    pub fn with_timeout(message: impl Into<String>, timeout_ms: u64) -> Self {
        Self {
            message: message.into(),
            timeout_ms: Some(timeout_ms),
        }
    }

    pub fn timeout_ms(&self) -> Option<u64> {
        self.timeout_ms
    }
}

impl std::fmt::Display for WhenTimeoutException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for WhenTimeoutException {}

impl LiteFlowException for WhenTimeoutException {
    fn message(&self) -> &str {
        &self.message
    }
}
