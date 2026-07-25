//! 对应 Java 类：com.yomahub.liteflow.exception.ComponentMethodDefineErrorException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 组件方法定义错误异常。
#[derive(Debug, Clone)]
pub struct ComponentMethodDefineErrorException {
    message: String,
    component_id: Option<String>,
    method_name: Option<String>,
}

impl ComponentMethodDefineErrorException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            component_id: None,
            method_name: None,
        }
    }

    pub fn with_detail(
        message: impl Into<String>,
        component_id: impl Into<String>,
        method_name: impl Into<String>,
    ) -> Self {
        Self {
            message: message.into(),
            component_id: Some(component_id.into()),
            method_name: Some(method_name.into()),
        }
    }
}

impl std::fmt::Display for ComponentMethodDefineErrorException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ComponentMethodDefineErrorException {}

impl LiteFlowException for ComponentMethodDefineErrorException {
    fn message(&self) -> &str {
        &self.message
    }
}
