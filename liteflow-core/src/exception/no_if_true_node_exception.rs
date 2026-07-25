//! 对应 Java 类：com.yomahub.liteflow.exception.NoIfTrueNodeException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 无 IF true 节点异常。
#[derive(Debug, Clone)]
pub struct NoIfTrueNodeException {
    message: String,
    chain_id: Option<String>,
}

impl NoIfTrueNodeException {
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

impl std::fmt::Display for NoIfTrueNodeException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NoIfTrueNodeException {}

impl LiteFlowException for NoIfTrueNodeException {
    fn message(&self) -> &str {
        &self.message
    }
}
