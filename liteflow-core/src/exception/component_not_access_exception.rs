//! 对应 Java 类：com.yomahub.liteflow.exception.ComponentNotAccessException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 组件不可访问异常。
#[derive(Debug, Clone)]
pub struct ComponentNotAccessException {
    message: String,
}

impl ComponentNotAccessException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ComponentNotAccessException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ComponentNotAccessException {}

impl LiteFlowException for ComponentNotAccessException {
    fn message(&self) -> &str {
        &self.message
    }
}
