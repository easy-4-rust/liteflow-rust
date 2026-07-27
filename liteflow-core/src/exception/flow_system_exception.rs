//! 对应 Java 类：com.yomahub.liteflow.exception.FlowSystemException
//!
//! 流程系统通用异常（兜底系统级错误）

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 FlowSystemException：流程系统通用异常（兜底系统级错误）
#[derive(Debug, Clone)]
pub struct FlowSystemException {
    /// 异常信息
    pub message: String,
}

impl FlowSystemException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// 返回异常信息。
    ///
    /// # 返回
    /// 当前异常持有的原始消息。对应 Java: `FlowSystemException#getMessage`。
    #[must_use]
    pub fn get_message(&self) -> &str {
        &self.message
    }

    /// 修改异常信息。
    ///
    /// 参数 `message` 对应 Java 同名参数。对应 Java:
    /// `FlowSystemException#setMessage`。
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}

impl fmt::Display for FlowSystemException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for FlowSystemException {}

impl From<FlowSystemException> for LiteflowError {
    fn from(e: FlowSystemException) -> Self {
        LiteflowError::Custom(e.message)
    }
}
