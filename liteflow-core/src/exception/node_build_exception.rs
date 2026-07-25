//! 对应 Java 类：com.yomahub.liteflow.exception.NodeBuildException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 节点构建异常。
#[derive(Debug, Clone)]
pub struct NodeBuildException {
    message: String,
    node_id: Option<String>,
}

impl NodeBuildException {
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

impl std::fmt::Display for NodeBuildException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NodeBuildException {}

impl LiteFlowException for NodeBuildException {
    fn message(&self) -> &str {
        &self.message
    }
}
