//! 对应 Java 类：com.yomahub.liteflow.exception.NullNodeTypeException
//!
//! 节点类型为空

use std::fmt;

/// 对应 NullNodeTypeException：节点类型为空
#[derive(Debug, Clone)]
pub struct NullNodeTypeException {
    /// 异常信息
    pub message: String,
}

impl NullNodeTypeException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for NullNodeTypeException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NullNodeTypeException {}
