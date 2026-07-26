//! 对应 Java 类：com.yomahub.liteflow.exception.NotSupportDeclException
//!
//! 不支持的声明方式（v2.16.0 新增）

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 NotSupportDeclException：不支持的声明方式（v2.16.0 新增）
#[derive(Debug, Clone)]
pub struct NotSupportDeclException {
    /// 异常信息
    pub message: String,
}

impl NotSupportDeclException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for NotSupportDeclException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NotSupportDeclException {}

impl From<NotSupportDeclException> for LiteflowError {
    fn from(e: NotSupportDeclException) -> Self {
        LiteflowError::NotSupportDecl(e.message)
    }
}
