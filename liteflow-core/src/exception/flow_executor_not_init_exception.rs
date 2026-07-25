//! 对应 Java 类：com.yomahub.liteflow.exception.FlowExecutorNotInitException
//!
//! FlowExecutor 未初始化

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 FlowExecutorNotInitException：FlowExecutor 未初始化
#[derive(Debug, Clone)]
pub struct FlowExecutorNotInitException {
    /// 异常信息
    pub message: String,
}

impl FlowExecutorNotInitException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for FlowExecutorNotInitException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for FlowExecutorNotInitException {}

impl From<FlowExecutorNotInitException> for LiteflowError {
    fn from(e: FlowExecutorNotInitException) -> Self {
        LiteflowError::FlowExecutorNotInit(e.message)
    }
}
