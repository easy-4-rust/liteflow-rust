//! 对应 Java 类：com.yomahub.liteflow.exception.DataNotFoundException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 数据未找到异常。
#[derive(Debug, Clone)]
pub struct DataNotFoundException {
    message: String,
}

impl DataNotFoundException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for DataNotFoundException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for DataNotFoundException {}

impl LiteFlowException for DataNotFoundException {
    fn message(&self) -> &str {
        &self.message
    }
}
