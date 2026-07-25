//! 对应 Java 类：com.yomahub.liteflow.exception.ChainNotImplementedException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 链未实现异常。
#[derive(Debug, Clone)]
pub struct ChainNotImplementedException {
    message: String,
}

impl ChainNotImplementedException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ChainNotImplementedException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ChainNotImplementedException {}

impl LiteFlowException for ChainNotImplementedException {
    fn message(&self) -> &str {
        &self.message
    }
}
