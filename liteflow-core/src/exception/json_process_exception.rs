//! 对应 Java 类：com.yomahub.liteflow.exception.JsonProcessException
//!
//! JSON 处理错误

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 JsonProcessException：JSON 处理错误
#[derive(Debug, Clone)]
pub struct JsonProcessException {
    /// 异常信息
    pub message: String,
}

impl JsonProcessException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for JsonProcessException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for JsonProcessException {}

impl From<JsonProcessException> for LiteflowError {
    fn from(e: JsonProcessException) -> Self {
        LiteflowError::Rule(e.message)
    }
}
