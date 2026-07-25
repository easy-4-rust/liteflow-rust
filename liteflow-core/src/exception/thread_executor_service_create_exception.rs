//! 对应 Java 类：com.yomahub.liteflow.exception.ThreadExecutorServiceCreateException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 线程执行器服务创建异常。
#[derive(Debug, Clone)]
pub struct ThreadExecutorServiceCreateException {
    message: String,
}

impl ThreadExecutorServiceCreateException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ThreadExecutorServiceCreateException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ThreadExecutorServiceCreateException {}

impl LiteFlowException for ThreadExecutorServiceCreateException {
    fn message(&self) -> &str {
        &self.message
    }
}
