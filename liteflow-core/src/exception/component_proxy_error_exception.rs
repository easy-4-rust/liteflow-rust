//! 对应 Java 类：com.yomahub.liteflow.exception.ComponentProxyErrorException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 组件代理错误异常。
#[derive(Debug, Clone)]
pub struct ComponentProxyErrorException {
    message: String,
}

impl ComponentProxyErrorException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ComponentProxyErrorException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ComponentProxyErrorException {}

impl LiteFlowException for ComponentProxyErrorException {
    fn message(&self) -> &str {
        &self.message
    }
}
