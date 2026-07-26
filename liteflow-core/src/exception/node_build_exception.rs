//! 对应 Java 类：com.yomahub.liteflow.exception.NodeBuildException
//!
//! 节点构建期错误（节点未注册等）

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 NodeBuildException：节点构建期错误（节点未注册等）
#[derive(Debug, Clone)]
pub struct NodeBuildException {
    /// 异常信息
    pub message: String,
}

impl NodeBuildException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for NodeBuildException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NodeBuildException {}

impl From<NodeBuildException> for LiteflowError {
    fn from(e: NodeBuildException) -> Self {
        LiteflowError::NodeBuild(e.message)
    }
}
