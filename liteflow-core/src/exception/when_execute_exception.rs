//! 对应 Java 类：com.yomahub.liteflow.exception.WhenExecuteException
//!
//! WHEN 并行执行错误

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 WhenExecuteException：WHEN 并行执行错误
#[derive(Debug, Clone)]
pub struct WhenExecuteException {
    /// 异常信息
    pub message: String,
}

impl WhenExecuteException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for WhenExecuteException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for WhenExecuteException {}

impl From<WhenExecuteException> for LiteflowError {
    fn from(e: WhenExecuteException) -> Self {
        LiteflowError::WhenExecute(e.message)
    }
}
