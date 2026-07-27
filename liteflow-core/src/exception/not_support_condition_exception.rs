//! 对应 Java 类：com.yomahub.liteflow.exception.NotSupportConditionException
//!
//! 不支持的条件类型

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 NotSupportConditionException：不支持的条件类型
#[derive(Debug, Clone)]
pub struct NotSupportConditionException {
    /// 异常信息
    pub message: String,
}

impl NotSupportConditionException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// 返回异常信息。
    ///
    /// # 返回
    /// 当前异常持有的原始消息。对应 Java: `NotSupportConditionException#getMessage`。
    #[must_use]
    pub fn get_message(&self) -> &str {
        &self.message
    }

    /// 修改异常信息。
    ///
    /// 参数 `message` 对应 Java 同名参数。对应 Java:
    /// `NotSupportConditionException#setMessage`。
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}

impl fmt::Display for NotSupportConditionException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NotSupportConditionException {}

impl From<NotSupportConditionException> for LiteflowError {
    fn from(e: NotSupportConditionException) -> Self {
        LiteflowError::NotSupportCondition(e.message)
    }
}
