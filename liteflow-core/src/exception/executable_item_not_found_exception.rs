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
        Self {
            message: message.into(),
        }
    }

    /// 返回异常信息。
    ///
    /// # 返回
    /// 当前异常持有的原始消息。对应 Java:
    /// `ExecutableItemNotFoundException#getMessage`。
    #[must_use]
    pub fn get_message(&self) -> &str {
        &self.message
    }

    /// 修改异常信息。
    ///
    /// 参数 `message` 对应 Java 同名参数。对应 Java:
    /// `ExecutableItemNotFoundException#setMessage`。
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}

impl Default for ExecutableItemNotFoundException {
    /// 创建消息为空的异常，对应 Java 无参构造器。
    fn default() -> Self {
        Self {
            message: String::new(),
        }
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
