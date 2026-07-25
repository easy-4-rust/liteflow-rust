//! 对应 Java 类：com.yomahub.liteflow.exception.NodeIdUnIllegalException
//!
//! 节点 ID 非法：必须以字母/下划线/$ 开头，仅由字母数字下划线$组成（v2.16.0 新增）

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 NodeIdUnIllegalException：节点 ID 非法：必须以字母/下划线/$ 开头，仅由字母数字下划线$组成（v2.16.0 新增）
#[derive(Debug, Clone)]
pub struct NodeIdUnIllegalException {
    /// 异常信息
    pub message: String,
}

impl NodeIdUnIllegalException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for NodeIdUnIllegalException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NodeIdUnIllegalException {}

impl From<NodeIdUnIllegalException> for LiteflowError {
    fn from(e: NodeIdUnIllegalException) -> Self {
        LiteflowError::NodeIdUnIllegal(e.message)
    }
}
