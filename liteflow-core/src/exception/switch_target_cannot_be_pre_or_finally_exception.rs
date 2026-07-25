//! 对应 Java 类：com.yomahub.liteflow.exception.SwitchTargetCannotBePreOrFinallyException

use crate::exception::lite_flow_exception::LiteFlowException;

/// SWITCH 目标不能为 PRE 或 FINALLY 异常。
#[derive(Debug, Clone)]
pub struct SwitchTargetCannotBePreOrFinallyException {
    message: String,
    node_id: Option<String>,
}

impl SwitchTargetCannotBePreOrFinallyException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            node_id: None,
        }
    }

    pub fn with_node_id(message: impl Into<String>, node_id: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            node_id: Some(node_id.into()),
        }
    }

    pub fn node_id(&self) -> Option<&str> {
        self.node_id.as_deref()
    }
}

impl std::fmt::Display for SwitchTargetCannotBePreOrFinallyException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SwitchTargetCannotBePreOrFinallyException {}

impl LiteFlowException for SwitchTargetCannotBePreOrFinallyException {
    fn message(&self) -> &str {
        &self.message
    }
}
