//! 对应 Java 类：com.yomahub.liteflow.exception.ChainEndException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 链结束异常，用于提前终止当前链的执行。
#[derive(Debug, Clone)]
pub struct ChainEndException {
    message: String,
    chain_id: Option<String>,
}

impl ChainEndException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            chain_id: None,
        }
    }

    pub fn with_chain(message: impl Into<String>, chain_id: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            chain_id: Some(chain_id.into()),
        }
    }

    pub fn chain_id(&self) -> Option<&str> {
        self.chain_id.as_deref()
    }
}

impl std::fmt::Display for ChainEndException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ChainEndException {}

impl LiteFlowException for ChainEndException {
    fn message(&self) -> &str {
        &self.message
    }
}
