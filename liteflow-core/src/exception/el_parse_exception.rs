//! 对应 Java 类：com.yomahub.liteflow.exception.ELParseException

use crate::exception::lite_flow_exception::LiteFlowException;

/// EL 表达式解析异常。
#[derive(Debug, Clone)]
pub struct ElParseException {
    message: String,
    el_expression: Option<String>,
}

impl ElParseException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            el_expression: None,
        }
    }

    pub fn with_el(message: impl Into<String>, el_expression: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            el_expression: Some(el_expression.into()),
        }
    }
}

impl std::fmt::Display for ElParseException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ElParseException {}

impl LiteFlowException for ElParseException {
    fn message(&self) -> &str {
        &self.message
    }
}
