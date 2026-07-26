//! 对应 Java 类：com.yomahub.liteflow.exception.ELParseException
//!
//! EL 表达式解析错误

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 ELParseException：EL 表达式解析错误
#[derive(Debug, Clone)]
pub struct ELParseException {
    /// 异常信息
    pub message: String,
}

impl ELParseException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ELParseException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ELParseException {}

impl From<ELParseException> for LiteflowError {
    fn from(e: ELParseException) -> Self {
        LiteflowError::Parse(e.message)
    }
}
