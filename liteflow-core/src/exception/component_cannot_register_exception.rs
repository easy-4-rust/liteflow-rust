//! 对应 Java 类：com.yomahub.liteflow.exception.ComponentCannotRegisterException
//!
//! 组件无法注册（注册流程非法）

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 ComponentCannotRegisterException：组件无法注册（注册流程非法）
#[derive(Debug, Clone)]
pub struct ComponentCannotRegisterException {
    /// 异常信息
    pub message: String,
}

impl ComponentCannotRegisterException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for ComponentCannotRegisterException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ComponentCannotRegisterException {}

impl From<ComponentCannotRegisterException> for LiteflowError {
    fn from(e: ComponentCannotRegisterException) -> Self {
        LiteflowError::ComponentCannotRegister(e.message)
    }
}
