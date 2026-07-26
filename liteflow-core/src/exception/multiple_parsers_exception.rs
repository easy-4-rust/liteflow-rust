//! 对应 Java 类：com.yomahub.liteflow.exception.MultipleParsersException
//!
//! 存在多个规则解析器（无法确定使用哪个）

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 MultipleParsersException：存在多个规则解析器（无法确定使用哪个）
#[derive(Debug, Clone)]
pub struct MultipleParsersException {
    /// 异常信息
    pub message: String,
}

impl MultipleParsersException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for MultipleParsersException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for MultipleParsersException {}

impl From<MultipleParsersException> for LiteflowError {
    fn from(e: MultipleParsersException) -> Self {
        LiteflowError::MultipleParsers(e.message)
    }
}
