//! 对应 Java 类：com.yomahub.liteflow.exception.SwitchTypeErrorException

use crate::exception::lite_flow_exception::LiteFlowException;

/// SWITCH 类型错误异常。
#[derive(Debug, Clone)]
pub struct SwitchTypeErrorException {
    message: String,
    node_id: Option<String>,
    expected_type: Option<String>,
}

impl SwitchTypeErrorException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            node_id: None,
            expected_type: None,
        }
    }

    pub fn with_detail(
        message: impl Into<String>,
        node_id: impl Into<String>,
        expected_type: impl Into<String>,
    ) -> Self {
        Self {
            message: message.into(),
            node_id: Some(node_id.into()),
            expected_type: Some(expected_type.into()),
        }
    }
}

impl std::fmt::Display for SwitchTypeErrorException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SwitchTypeErrorException {}

impl LiteFlowException for SwitchTypeErrorException {
    fn message(&self) -> &str {
        &self.message
    }
}
