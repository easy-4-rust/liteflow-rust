//! 对应 Java 类：com.yomahub.liteflow.exception.ParallelExecutorCreateException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 并行执行器创建异常。
#[derive(Debug, Clone)]
pub struct ParallelExecutorCreateException {
    message: String,
}

impl ParallelExecutorCreateException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ParallelExecutorCreateException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParallelExecutorCreateException {}

impl LiteFlowException for ParallelExecutorCreateException {
    fn message(&self) -> &str {
        &self.message
    }
}
