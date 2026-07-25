//! 对应 Java 类：com.yomahub.liteflow.exception.NoSuchContextBeanException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 无此上下文 Bean 异常。
#[derive(Debug, Clone)]
pub struct NoSuchContextBeanException {
    message: String,
}

impl NoSuchContextBeanException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for NoSuchContextBeanException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NoSuchContextBeanException {}

impl LiteFlowException for NoSuchContextBeanException {
    fn message(&self) -> &str {
        &self.message
    }
}
