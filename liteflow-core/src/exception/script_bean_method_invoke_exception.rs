//! 对应 Java 类：com.yomahub.liteflow.exception.ScriptBeanMethodInvokeException
//!
//! 脚本 Bean 方法调用错误

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 ScriptBeanMethodInvokeException：脚本 Bean 方法调用错误
#[derive(Debug, Clone)]
pub struct ScriptBeanMethodInvokeException {
    /// 异常信息
    pub message: String,
}

impl ScriptBeanMethodInvokeException {
    /// 使用指定消息创建异常。
    ///
    /// 参数 `message` 对应 Java 同名构造参数。对应 Java:
    /// `ScriptBeanMethodInvokeException#ScriptBeanMethodInvokeException(String)`。
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// 返回异常信息。
    ///
    /// # 返回
    /// 当前异常持有的原始消息。对应 Java:
    /// `ScriptBeanMethodInvokeException#getMessage`。
    #[must_use]
    pub fn get_message(&self) -> &str {
        &self.message
    }

    /// 修改异常信息。
    ///
    /// 参数 `message` 对应 Java 同名参数。对应 Java:
    /// `ScriptBeanMethodInvokeException#setMessage`。
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}

impl fmt::Display for ScriptBeanMethodInvokeException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ScriptBeanMethodInvokeException {}

impl From<ScriptBeanMethodInvokeException> for LiteflowError {
    fn from(e: ScriptBeanMethodInvokeException) -> Self {
        // Java 对象不携带节点字段，统一错误也必须逐字保留调用方消息。
        LiteflowError::Custom(e.message)
    }
}
