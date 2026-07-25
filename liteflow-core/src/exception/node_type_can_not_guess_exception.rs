//! 对应 Java 类：com.yomahub.liteflow.exception.NodeTypeCanNotGuessException
//!
//! 节点类型无法推断

use std::fmt;

/// 对应 NodeTypeCanNotGuessException：节点类型无法推断
#[derive(Debug, Clone)]
pub struct NodeTypeCanNotGuessException {
    /// 异常信息
    pub message: String,
}

impl NodeTypeCanNotGuessException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for NodeTypeCanNotGuessException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NodeTypeCanNotGuessException {}
