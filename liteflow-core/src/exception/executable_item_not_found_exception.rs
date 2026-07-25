//! 对应 Java 类：com.yomahub.liteflow.exception.ExecutableItemNotFoundException
//!
//! 可执行项（节点/条件）未找到

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 ExecutableItemNotFoundException：可执行项（节点/条件）未找到
#[derive(Debug, Clone)]
pub struct ExecutableItemNotFoundException {
    /// 异常信息
    pub message: String,
}

impl ExecutableItemNotFoundException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for ExecutableItemNotFoundException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ExecutableItemNotFoundException {}

impl From<ExecutableItemNotFoundException> for LiteflowError {
    fn from(e: ExecutableItemNotFoundException) -> Self {
        LiteflowError::NodeNotFound(e.message)
    }
}
