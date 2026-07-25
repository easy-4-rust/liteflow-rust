//! 对应 Java 类：com.yomahub.liteflow.exception.RequestIdGeneratorException
//!
//! requestId 生成器错误

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 RequestIdGeneratorException：requestId 生成器错误
#[derive(Debug, Clone)]
pub struct RequestIdGeneratorException {
    /// 异常信息
    pub message: String,
}

impl RequestIdGeneratorException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for RequestIdGeneratorException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RequestIdGeneratorException {}

impl From<RequestIdGeneratorException> for LiteflowError {
    fn from(e: RequestIdGeneratorException) -> Self {
        LiteflowError::RequestIdGenerator(e.message)
    }
}
