//! 对应 Java 类：com.yomahub.liteflow.exception.IfTypeErrorException
//!
//! IF 节点返回类型错误（应返回布尔值）

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// IF 条件返回类型错误时使用的可变消息异常。
///
/// 对应 Java: `com.yomahub.liteflow.exception.IfTypeErrorException`。
#[derive(Debug, Clone)]
pub struct IfTypeErrorException {
    /// 异常信息
    pub message: String,
}

impl IfTypeErrorException {
    /// 使用指定消息创建异常。
    ///
    /// 参数 `message` 对应 Java 同名构造参数。对应 Java:
    /// `IfTypeErrorException#IfTypeErrorException(String)`。
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// 返回异常信息。
    ///
    /// # 返回
    /// 当前异常持有的原始消息。对应 Java: `IfTypeErrorException#getMessage`。
    #[must_use]
    pub fn get_message(&self) -> &str {
        &self.message
    }

    /// 修改异常信息。
    ///
    /// 参数 `message` 对应 Java 同名参数。对应 Java:
    /// `IfTypeErrorException#setMessage`。
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}

impl fmt::Display for IfTypeErrorException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for IfTypeErrorException {}

impl From<IfTypeErrorException> for LiteflowError {
    fn from(e: IfTypeErrorException) -> Self {
        // Java 允许调用者传入任意消息，统一错误转换必须逐字保留该诊断文本。
        LiteflowError::Custom(e.message)
    }
}
