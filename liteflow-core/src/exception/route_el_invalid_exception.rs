//! 对应 Java 类：com.yomahub.liteflow.exception.RouteELInvalidException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 路由 EL 无效异常。
#[derive(Debug, Clone)]
pub struct RouteElInvalidException {
    message: String,
}

impl RouteElInvalidException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for RouteElInvalidException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RouteElInvalidException {}

impl LiteFlowException for RouteElInvalidException {
    fn message(&self) -> &str {
        &self.message
    }
}
