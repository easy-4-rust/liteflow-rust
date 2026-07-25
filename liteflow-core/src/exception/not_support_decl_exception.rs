//! 对应 Java 类：com.yomahub.liteflow.exception.NotSupportDeclException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 不支持的声明异常。
#[derive(Debug, Clone)]
pub struct NotSupportDeclException {
    message: String,
}

impl NotSupportDeclException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for NotSupportDeclException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NotSupportDeclException {}

impl LiteFlowException for NotSupportDeclException {
    fn message(&self) -> &str {
        &self.message
    }
}
