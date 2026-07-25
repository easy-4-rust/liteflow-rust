//! 对应 Java 类：com.yomahub.liteflow.exception.NullNodeTypeException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 节点类型为空异常。
#[derive(Debug, Clone)]
pub struct NullNodeTypeException {
    message: String,
}

impl NullNodeTypeException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for NullNodeTypeException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NullNodeTypeException {}

impl LiteFlowException for NullNodeTypeException {
    fn message(&self) -> &str {
        &self.message
    }
}
