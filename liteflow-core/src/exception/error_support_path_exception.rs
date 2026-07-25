//! 对应 Java 类：com.yomahub.liteflow.exception.ErrorSupportPathException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 错误支持路径异常。
#[derive(Debug, Clone)]
pub struct ErrorSupportPathException {
    message: String,
}

impl ErrorSupportPathException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ErrorSupportPathException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ErrorSupportPathException {}

impl LiteFlowException for ErrorSupportPathException {
    fn message(&self) -> &str {
        &self.message
    }
}
