//! 对应 Java 类：com.yomahub.liteflow.exception.ComponentCannotRegisterException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 组件无法注册异常。
#[derive(Debug, Clone)]
pub struct ComponentCannotRegisterException {
    message: String,
}

impl ComponentCannotRegisterException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ComponentCannotRegisterException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ComponentCannotRegisterException {}

impl LiteFlowException for ComponentCannotRegisterException {
    fn message(&self) -> &str {
        &self.message
    }
}
