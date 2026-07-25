//! 对应 Java 类：com.yomahub.liteflow.exception.NodeClassNotFoundException
//!
//! 节点类未找到

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 NodeClassNotFoundException：节点类未找到
#[derive(Debug, Clone)]
pub struct NodeClassNotFoundException {
    /// 异常信息
    pub message: String,
}

impl NodeClassNotFoundException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for NodeClassNotFoundException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NodeClassNotFoundException {}

impl From<NodeClassNotFoundException> for LiteflowError {
    fn from(e: NodeClassNotFoundException) -> Self {
        LiteflowError::NodeClassNotFound(e.message)
    }
}
