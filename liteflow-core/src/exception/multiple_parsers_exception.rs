//! 对应 Java 类：com.yomahub.liteflow.exception.MultipleParsersException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 多个解析器异常。
#[derive(Debug, Clone)]
pub struct MultipleParsersException {
    message: String,
}

impl MultipleParsersException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for MultipleParsersException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for MultipleParsersException {}

impl LiteFlowException for MultipleParsersException {
    fn message(&self) -> &str {
        &self.message
    }
}
