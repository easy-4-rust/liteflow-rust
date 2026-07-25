//! 对应 Java 类：com.yomahub.liteflow.exception.ExecutableItemNotFoundException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 可执行项未找到异常。
#[derive(Debug, Clone)]
pub struct ExecutableItemNotFoundException {
    message: String,
    item_id: Option<String>,
}

impl ExecutableItemNotFoundException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            item_id: None,
        }
    }

    pub fn with_item_id(message: impl Into<String>, item_id: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            item_id: Some(item_id.into()),
        }
    }

    pub fn item_id(&self) -> Option<&str> {
        self.item_id.as_deref()
    }
}

impl std::fmt::Display for ExecutableItemNotFoundException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ExecutableItemNotFoundException {}

impl LiteFlowException for ExecutableItemNotFoundException {
    fn message(&self) -> &str {
        &self.message
    }
}
