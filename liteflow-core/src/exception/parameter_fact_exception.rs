//! 对应 Java 类：com.yomahub.liteflow.exception.ParameterFactException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 参数事实异常。
#[derive(Debug, Clone)]
pub struct ParameterFactException {
    message: String,
}

impl ParameterFactException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ParameterFactException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParameterFactException {}

impl LiteFlowException for ParameterFactException {
    fn message(&self) -> &str {
        &self.message
    }
}
