//! 对应 Java 类：com.yomahub.liteflow.exception.NotSupportConditionException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 不支持的条件异常。
#[derive(Debug, Clone)]
pub struct NotSupportConditionException {
    message: String,
}

impl NotSupportConditionException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for NotSupportConditionException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NotSupportConditionException {}

impl LiteFlowException for NotSupportConditionException {
    fn message(&self) -> &str {
        &self.message
    }
}
