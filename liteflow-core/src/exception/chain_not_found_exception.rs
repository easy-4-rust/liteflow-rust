//! 对应 Java 类：com.yomahub.liteflow.exception.ChainNotFoundException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 链未找到异常。
#[derive(Debug, Clone)]
pub struct ChainNotFoundException {
    message: String,
    chain_id: Option<String>,
}

impl ChainNotFoundException {
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

impl std::fmt::Display for ChainNotFoundException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ChainNotFoundException {}

impl LiteFlowException for ChainNotFoundException {
    fn message(&self) -> &str {
        &self.message
    }
}
