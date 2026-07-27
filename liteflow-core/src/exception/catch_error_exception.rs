//! 对应 Java 类：com.yomahub.liteflow.exception.CatchErrorException
//!
//! 组件执行捕获到错误时抛出（CATCH 语义载体）

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 CatchErrorException：组件执行捕获到错误时抛出（CATCH 语义载体）
#[derive(Debug, Clone)]
pub struct CatchErrorException {
    /// 异常信息
    pub message: String,
}

impl CatchErrorException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// 返回异常信息。
    ///
    /// # 返回
    /// 当前异常持有的原始消息。对应 Java: `CatchErrorException#getMessage`。
    #[must_use]
    pub fn get_message(&self) -> &str {
        &self.message
    }

    /// 修改异常信息。
    ///
    /// 参数 `message` 对应 Java 同名参数。对应 Java:
    /// `CatchErrorException#setMessage`。
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}

impl fmt::Display for CatchErrorException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CatchErrorException {}

impl From<CatchErrorException> for LiteflowError {
    fn from(e: CatchErrorException) -> Self {
        LiteflowError::CatchError(e.message)
    }
}
