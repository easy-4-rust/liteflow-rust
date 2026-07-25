//! 对应 Java 类：com.yomahub.liteflow.exception.EmptyConditionValueException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 条件值为空异常。
#[derive(Debug, Clone)]
pub struct EmptyConditionValueException {
    message: String,
}

impl EmptyConditionValueException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for EmptyConditionValueException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for EmptyConditionValueException {}

impl LiteFlowException for EmptyConditionValueException {
    fn message(&self) -> &str {
        &self.message
    }
}
