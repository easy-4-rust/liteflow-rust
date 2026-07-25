//! 对应 Java 类：com.yomahub.liteflow.exception.ThreadExecutorServiceCreateException
//!
//! 线程池创建错误

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 ThreadExecutorServiceCreateException：线程池创建错误
#[derive(Debug, Clone)]
pub struct ThreadExecutorServiceCreateException {
    /// 异常信息
    pub message: String,
}

impl ThreadExecutorServiceCreateException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for ThreadExecutorServiceCreateException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ThreadExecutorServiceCreateException {}

impl From<ThreadExecutorServiceCreateException> for LiteflowError {
    fn from(e: ThreadExecutorServiceCreateException) -> Self {
        LiteflowError::ThreadExecutorServiceCreate(e.message)
    }
}
