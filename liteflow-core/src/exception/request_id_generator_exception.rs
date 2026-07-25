//! 对应 Java 类：com.yomahub.liteflow.exception.RequestIdGeneratorException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 请求 ID 生成器异常。
#[derive(Debug, Clone)]
pub struct RequestIdGeneratorException {
    message: String,
}

impl RequestIdGeneratorException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for RequestIdGeneratorException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RequestIdGeneratorException {}

impl LiteFlowException for RequestIdGeneratorException {
    fn message(&self) -> &str {
        &self.message
    }
}
