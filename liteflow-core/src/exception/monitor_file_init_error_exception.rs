//! 对应 Java 类：com.yomahub.liteflow.exception.MonitorFileInitErrorException
//!
//! 监控文件初始化错误

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 MonitorFileInitErrorException：监控文件初始化错误
#[derive(Debug, Clone)]
pub struct MonitorFileInitErrorException {
    /// 异常信息
    pub message: String,
}

impl MonitorFileInitErrorException {
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
    /// `MonitorFileInitErrorException#getMessage`。
    #[must_use]
    pub fn get_message(&self) -> &str {
        &self.message
    }

    /// 修改异常信息。
    ///
    /// 参数 `message` 对应 Java 同名参数。对应 Java:
    /// `MonitorFileInitErrorException#setMessage`。
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}

impl fmt::Display for MonitorFileInitErrorException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for MonitorFileInitErrorException {}

impl From<MonitorFileInitErrorException> for LiteflowError {
    fn from(e: MonitorFileInitErrorException) -> Self {
        LiteflowError::MonitorFileInitError(e.message)
    }
}
