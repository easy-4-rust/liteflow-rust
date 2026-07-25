//! 对应 Java 类：com.yomahub.liteflow.exception.ChainDuplicateException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 链重复异常。
#[derive(Debug, Clone)]
pub struct ChainDuplicateException {
    message: String,
}

impl ChainDuplicateException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ChainDuplicateException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ChainDuplicateException {}

impl LiteFlowException for ChainDuplicateException {
    fn message(&self) -> &str {
        &self.message
    }
}
