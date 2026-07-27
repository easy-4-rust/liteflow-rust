//! 对应 Java 类：com.yomahub.liteflow.exception.RouteELInvalidException
//!
//! 路由 EL 非法（v2.16.0 新增）

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 RouteELInvalidException：路由 EL 非法（v2.16.0 新增）
#[derive(Debug, Clone)]
pub struct RouteELInvalidException {
    /// 异常信息
    pub message: String,
}

impl RouteELInvalidException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// 返回异常信息。
    ///
    /// # 返回
    /// 当前异常持有的原始消息。对应 Java: `RouteELInvalidException#getMessage`。
    #[must_use]
    pub fn get_message(&self) -> &str {
        &self.message
    }

    /// 修改异常信息。
    ///
    /// 参数 `message` 对应 Java 同名参数。对应 Java:
    /// `RouteELInvalidException#setMessage`。
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}

impl fmt::Display for RouteELInvalidException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RouteELInvalidException {}

impl From<RouteELInvalidException> for LiteflowError {
    fn from(e: RouteELInvalidException) -> Self {
        LiteflowError::RouteELInvalid(e.message)
    }
}
