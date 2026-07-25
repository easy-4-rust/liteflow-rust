//! 对应 Java 类：com.yomahub.liteflow.exception.NullParamException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 参数为空异常。
#[derive(Debug, Clone)]
pub struct NullParamException {
    message: String,
}

impl NullParamException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for NullParamException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NullParamException {}

impl LiteFlowException for NullParamException {
    fn message(&self) -> &str {
        &self.message
    }
}
