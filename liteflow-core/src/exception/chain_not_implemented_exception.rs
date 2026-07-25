//! 对应 Java 类：com.yomahub.liteflow.exception.ChainNotImplementedException
//!
//! 抽象链未被继承实现（v2.16.0 新增）

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 ChainNotImplementedException：抽象链未被继承实现（v2.16.0 新增）
#[derive(Debug, Clone)]
pub struct ChainNotImplementedException {
    /// 异常信息
    pub message: String,
}

impl ChainNotImplementedException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for ChainNotImplementedException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ChainNotImplementedException {}

impl From<ChainNotImplementedException> for LiteflowError {
    fn from(e: ChainNotImplementedException) -> Self {
        LiteflowError::ChainNotImplemented(e.message)
    }
}
