//! 对应 Java 类：com.yomahub.liteflow.exception.AndOrConditionException
//!
//! AND/OR 条件错误（v2.16.0 新增）

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 AndOrConditionException：AND/OR 条件错误（v2.16.0 新增）
#[derive(Debug, Clone)]
pub struct AndOrConditionException {
    /// 异常信息
    pub message: String,
}

impl AndOrConditionException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// 返回异常信息。
    ///
    /// # 返回
    /// 当前异常持有的原始消息。对应 Java: `AndOrConditionException#getMessage`。
    #[must_use]
    pub fn get_message(&self) -> &str {
        &self.message
    }

    /// 修改异常信息。
    ///
    /// 参数 `message` 对应 Java 同名参数。对应 Java:
    /// `AndOrConditionException#setMessage`。
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}

impl fmt::Display for AndOrConditionException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AndOrConditionException {}

impl From<AndOrConditionException> for LiteflowError {
    fn from(e: AndOrConditionException) -> Self {
        LiteflowError::AndOrCondition(e.message)
    }
}
