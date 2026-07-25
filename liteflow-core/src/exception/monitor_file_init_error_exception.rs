//! 对应 Java 类：com.yomahub.liteflow.exception.MonitorFileInitErrorException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 监控文件初始化错误异常。
#[derive(Debug, Clone)]
pub struct MonitorFileInitErrorException {
    message: String,
}

impl MonitorFileInitErrorException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for MonitorFileInitErrorException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for MonitorFileInitErrorException {}

impl LiteFlowException for MonitorFileInitErrorException {
    fn message(&self) -> &str {
        &self.message
    }
}
