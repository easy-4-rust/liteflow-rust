//! 对应 Java 类：com.yomahub.liteflow.exception.NodeTypeCanNotGuessException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 节点类型无法推测异常。
#[derive(Debug, Clone)]
pub struct NodeTypeCanNotGuessException {
    message: String,
}

impl NodeTypeCanNotGuessException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for NodeTypeCanNotGuessException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NodeTypeCanNotGuessException {}

impl LiteFlowException for NodeTypeCanNotGuessException {
    fn message(&self) -> &str {
        &self.message
    }
}
