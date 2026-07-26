//! 对应 Java 类：com.yomahub.liteflow.exception.ChainNotFoundException
//!
//! 根据 id/name 未找到对应链

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 ChainNotFoundException：根据 id/name 未找到对应链
#[derive(Debug, Clone)]
pub struct ChainNotFoundException {
    /// 异常信息
    pub message: String,
}

impl ChainNotFoundException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ChainNotFoundException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ChainNotFoundException {}

impl From<ChainNotFoundException> for LiteflowError {
    fn from(e: ChainNotFoundException) -> Self {
        LiteflowError::ChainNotFound(e.message)
    }
}
