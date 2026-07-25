//! 对应 Java 类：com.yomahub.liteflow.exception.RouteChainNotFoundException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 路由链未找到异常。
#[derive(Debug, Clone)]
pub struct RouteChainNotFoundException {
    message: String,
    route_key: Option<String>,
}

impl RouteChainNotFoundException {
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

impl std::fmt::Display for RouteChainNotFoundException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RouteChainNotFoundException {}

impl LiteFlowException for RouteChainNotFoundException {
    fn message(&self) -> &str {
        &self.message
    }
}
