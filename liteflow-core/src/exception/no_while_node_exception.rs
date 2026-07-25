//! 对应 Java 类：com.yomahub.liteflow.exception.NoWhileNodeException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 无 WHILE 节点异常。
#[derive(Debug, Clone)]
pub struct NoWhileNodeException {
    message: String,
    chain_id: Option<String>,
}

impl NoWhileNodeException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            chain_id: None,
        }
    }

    pub fn with_chain_id(message: impl Into<String>, chain_id: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            chain_id: Some(chain_id.into()),
        }
    }

    pub fn chain_id(&self) -> Option<&str> {
        self.chain_id.as_deref()
    }
}

impl std::fmt::Display for NoWhileNodeException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NoWhileNodeException {}

impl LiteFlowException for NoWhileNodeException {
    fn message(&self) -> &str {
        &self.message
    }
}
