//! 对应 Java 类：com.yomahub.liteflow.exception.ParserCannotFindException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 解析器无法找到异常。
#[derive(Debug, Clone)]
pub struct ParserCannotFindException {
    message: String,
}

impl ParserCannotFindException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ParserCannotFindException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParserCannotFindException {}

impl LiteFlowException for ParserCannotFindException {
    fn message(&self) -> &str {
        &self.message
    }
}
