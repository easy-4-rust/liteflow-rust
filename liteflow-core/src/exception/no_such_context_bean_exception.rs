//! 对应 Java 类：com.yomahub.liteflow.exception.NoSuchContextBeanException
//!
//! 指定的上下文 Bean 不存在

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 NoSuchContextBeanException：指定的上下文 Bean 不存在
#[derive(Debug, Clone)]
pub struct NoSuchContextBeanException {
    /// 异常信息
    pub message: String,
}

impl NoSuchContextBeanException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for NoSuchContextBeanException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NoSuchContextBeanException {}

impl From<NoSuchContextBeanException> for LiteflowError {
    fn from(e: NoSuchContextBeanException) -> Self {
        LiteflowError::NoSuchContextBean(e.message)
    }
}
