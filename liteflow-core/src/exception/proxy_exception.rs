//! 对应 Java 类：com.yomahub.liteflow.exception.ProxyException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 代理异常。
#[derive(Debug, Clone)]
pub struct ProxyException {
    message: String,
}

impl ProxyException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ProxyException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ProxyException {}

impl LiteFlowException for ProxyException {
    fn message(&self) -> &str {
        &self.message
    }
}
