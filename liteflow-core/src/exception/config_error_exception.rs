//! 对应 Java 类：com.yomahub.liteflow.exception.ConfigErrorException
//!
//! 规则配置错误

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 ConfigErrorException：规则配置错误
#[derive(Debug, Clone)]
pub struct ConfigErrorException {
    /// 异常信息
    pub message: String,
}

impl ConfigErrorException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ConfigErrorException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ConfigErrorException {}

impl From<ConfigErrorException> for LiteflowError {
    fn from(e: ConfigErrorException) -> Self {
        LiteflowError::Rule(e.message)
    }
}
