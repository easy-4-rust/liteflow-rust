//! 对应 Java 类：com.yomahub.liteflow.exception.ChainDuplicateException
//!
//! 链重复定义（同一 chain id/name 被重复注册）

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 ChainDuplicateException：链重复定义（同一 chain id/name 被重复注册）
#[derive(Debug, Clone)]
pub struct ChainDuplicateException {
    /// 异常信息
    pub message: String,
}

impl ChainDuplicateException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ChainDuplicateException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ChainDuplicateException {}

impl From<ChainDuplicateException> for LiteflowError {
    fn from(e: ChainDuplicateException) -> Self {
        LiteflowError::ChainDuplicate(e.message)
    }
}
