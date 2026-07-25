//! 对应 Java 类：com.yomahub.liteflow.exception.FallbackCmpNotFoundException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 降级组件未找到异常。
#[derive(Debug, Clone)]
pub struct FallbackCmpNotFoundException {
    message: String,
}

impl FallbackCmpNotFoundException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for FallbackCmpNotFoundException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for FallbackCmpNotFoundException {}

impl LiteFlowException for FallbackCmpNotFoundException {
    fn message(&self) -> &str {
        &self.message
    }
}
