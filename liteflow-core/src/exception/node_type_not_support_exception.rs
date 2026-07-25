//! 对应 Java 类：com.yomahub.liteflow.exception.NodeTypeNotSupportException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 节点类型不支持异常。
#[derive(Debug, Clone)]
pub struct NodeTypeNotSupportException {
    message: String,
}

impl NodeTypeNotSupportException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for NodeTypeNotSupportException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NodeTypeNotSupportException {}

impl LiteFlowException for NodeTypeNotSupportException {
    fn message(&self) -> &str {
        &self.message
    }
}
