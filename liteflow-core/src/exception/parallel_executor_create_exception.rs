//! 对应 Java 类：com.yomahub.liteflow.exception.ParallelExecutorCreateException
//!
//! 并行执行器创建错误（v2.16.0 新增）

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 ParallelExecutorCreateException：并行执行器创建错误（v2.16.0 新增）
#[derive(Debug, Clone)]
pub struct ParallelExecutorCreateException {
    /// 异常信息
    pub message: String,
}

impl ParallelExecutorCreateException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for ParallelExecutorCreateException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParallelExecutorCreateException {}

impl From<ParallelExecutorCreateException> for LiteflowError {
    fn from(e: ParallelExecutorCreateException) -> Self {
        LiteflowError::ParallelExecutorCreate(e.message)
    }
}
