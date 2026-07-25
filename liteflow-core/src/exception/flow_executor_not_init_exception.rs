//! 对应 Java 类：com.yomahub.liteflow.exception.FlowExecutorNotInitException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 流程执行器未初始化异常。
#[derive(Debug, Clone)]
pub struct FlowExecutorNotInitException {
    message: String,
}

impl FlowExecutorNotInitException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for FlowExecutorNotInitException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for FlowExecutorNotInitException {}

impl LiteFlowException for FlowExecutorNotInitException {
    fn message(&self) -> &str {
        &self.message
    }
}
