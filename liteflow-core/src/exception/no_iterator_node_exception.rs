//! 对应 Java 类：com.yomahub.liteflow.exception.NoIteratorNodeException
//!
//! ITERATOR 条件中缺少迭代节点

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 NoIteratorNodeException：ITERATOR 条件中缺少迭代节点
#[derive(Debug, Clone)]
pub struct NoIteratorNodeException {
    /// 异常信息
    pub message: String,
}

impl NoIteratorNodeException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for NoIteratorNodeException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NoIteratorNodeException {}

impl From<NoIteratorNodeException> for LiteflowError {
    fn from(_e: NoIteratorNodeException) -> Self {
        LiteflowError::NoIteratorNode
    }
}
