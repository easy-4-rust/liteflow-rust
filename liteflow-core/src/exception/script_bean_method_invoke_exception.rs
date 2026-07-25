//! 对应 Java 类：com.yomahub.liteflow.exception.ScriptBeanMethodInvokeException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 脚本 Bean 方法调用异常。
#[derive(Debug, Clone)]
pub struct ScriptBeanMethodInvokeException {
    message: String,
    bean_name: Option<String>,
    method_name: Option<String>,
}

impl ScriptBeanMethodInvokeException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            bean_name: None,
            method_name: None,
        }
    }

    pub fn with_detail(
        message: impl Into<String>,
        bean_name: impl Into<String>,
        method_name: impl Into<String>,
    ) -> Self {
        Self {
            message: message.into(),
            bean_name: Some(bean_name.into()),
            method_name: Some(method_name.into()),
        }
    }
}

impl std::fmt::Display for ScriptBeanMethodInvokeException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ScriptBeanMethodInvokeException {}

impl LiteFlowException for ScriptBeanMethodInvokeException {
    fn message(&self) -> &str {
        &self.message
    }
}
