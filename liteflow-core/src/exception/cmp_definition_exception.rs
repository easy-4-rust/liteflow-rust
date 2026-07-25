//! 对应 Java 类：com.yomahub.liteflow.exception.CmpDefinitionException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 组件定义异常。
#[derive(Debug, Clone)]
pub struct CmpDefinitionException {
    message: String,
}

impl CmpDefinitionException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for CmpDefinitionException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CmpDefinitionException {}

impl LiteFlowException for CmpDefinitionException {
    fn message(&self) -> &str {
        &self.message
    }
}
