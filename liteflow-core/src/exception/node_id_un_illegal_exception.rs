//! 对应 Java 类：com.yomahub.liteflow.exception.NodeIdUnIllegalException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 节点 ID 不合法异常。
#[derive(Debug, Clone)]
pub struct NodeIdUnIllegalException {
    message: String,
    node_id: Option<String>,
}

impl NodeIdUnIllegalException {
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

impl std::fmt::Display for NodeIdUnIllegalException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NodeIdUnIllegalException {}

impl LiteFlowException for NodeIdUnIllegalException {
    fn message(&self) -> &str {
        &self.message
    }
}
