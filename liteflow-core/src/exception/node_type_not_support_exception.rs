//! 对应 Java 类：com.yomahub.liteflow.exception.NodeTypeNotSupportException
//!
//! 节点类型不支持

use std::fmt;

/// 对应 NodeTypeNotSupportException：节点类型不支持
#[derive(Debug, Clone)]
pub struct NodeTypeNotSupportException {
    /// 异常信息
    pub message: String,
}

impl NodeTypeNotSupportException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for NodeTypeNotSupportException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NodeTypeNotSupportException {}
