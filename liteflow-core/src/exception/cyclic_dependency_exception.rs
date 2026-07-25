//! 对应 Java 类：com.yomahub.liteflow.exception.CyclicDependencyException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 循环依赖异常。
#[derive(Debug, Clone)]
pub struct CyclicDependencyException {
    message: String,
}

impl CyclicDependencyException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for CyclicDependencyException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CyclicDependencyException {}

impl LiteFlowException for CyclicDependencyException {
    fn message(&self) -> &str {
        &self.message
    }
}
