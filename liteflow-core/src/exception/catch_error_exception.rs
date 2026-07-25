//! 对应 Java 类：com.yomahub.liteflow.exception.CatchErrorException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 捕获错误异常。
#[derive(Debug, Clone)]
pub struct CatchErrorException {
    message: String,
}

impl CatchErrorException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for CatchErrorException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CatchErrorException {}

impl LiteFlowException for CatchErrorException {
    fn message(&self) -> &str {
        &self.message
    }
}
