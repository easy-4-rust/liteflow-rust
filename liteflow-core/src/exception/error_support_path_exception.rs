//! 对应 Java 类：com.yomahub.liteflow.exception.ErrorSupportPathException
//!
//! 不支持的规则路径（规则源路径非法）

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 ErrorSupportPathException：不支持的规则路径（规则源路径非法）
#[derive(Debug, Clone)]
pub struct ErrorSupportPathException {
    /// 异常信息
    pub message: String,
}

impl ErrorSupportPathException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for ErrorSupportPathException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ErrorSupportPathException {}

impl From<ErrorSupportPathException> for LiteflowError {
    fn from(e: ErrorSupportPathException) -> Self {
        LiteflowError::ErrorSupportPath(e.message)
    }
}
