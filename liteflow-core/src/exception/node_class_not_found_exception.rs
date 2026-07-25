//! 对应 Java 类：com.yomahub.liteflow.exception.NodeClassNotFoundException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 节点类未找到异常。
#[derive(Debug, Clone)]
pub struct NodeClassNotFoundException {
    message: String,
}

impl NodeClassNotFoundException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for NodeClassNotFoundException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NodeClassNotFoundException {}

impl LiteFlowException for NodeClassNotFoundException {
    fn message(&self) -> &str {
        &self.message
    }
}
