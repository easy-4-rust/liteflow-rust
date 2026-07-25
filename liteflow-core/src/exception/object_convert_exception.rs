//! 对应 Java 类：com.yomahub.liteflow.exception.ObjectConvertException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 对象转换异常。
#[derive(Debug, Clone)]
pub struct ObjectConvertException {
    message: String,
}

impl ObjectConvertException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ObjectConvertException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ObjectConvertException {}

impl LiteFlowException for ObjectConvertException {
    fn message(&self) -> &str {
        &self.message
    }
}
