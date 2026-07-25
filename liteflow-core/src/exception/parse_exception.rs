//! 对应 Java 类：com.yomahub.liteflow.exception.ParseException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 解析异常。
#[derive(Debug, Clone)]
pub struct ParseException {
    message: String,
    source: Option<String>,
}

impl ParseException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    pub fn with_source(message: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: Some(source.into()),
        }
    }
}

impl std::fmt::Display for ParseException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseException {}

impl LiteFlowException for ParseException {
    fn message(&self) -> &str {
        &self.message
    }
}
