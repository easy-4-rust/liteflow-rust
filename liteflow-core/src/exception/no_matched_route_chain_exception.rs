//! 对应 Java 类：com.yomahub.liteflow.exception.NoMatchedRouteChainException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 无匹配路由链异常。
#[derive(Debug, Clone)]
pub struct NoMatchedRouteChainException {
    message: String,
    route_key: Option<String>,
}

impl NoMatchedRouteChainException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            route_key: None,
        }
    }

    pub fn with_route_key(message: impl Into<String>, route_key: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            route_key: Some(route_key.into()),
        }
    }

    pub fn route_key(&self) -> Option<&str> {
        self.route_key.as_deref()
    }
}

impl std::fmt::Display for NoMatchedRouteChainException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NoMatchedRouteChainException {}

impl LiteFlowException for NoMatchedRouteChainException {
    fn message(&self) -> &str {
        &self.message
    }
}
