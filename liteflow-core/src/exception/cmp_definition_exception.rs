//! 对应 Java 类：com.yomahub.liteflow.exception.CmpDefinitionException
//!
//! 组件定义错误（v2.16.0 新增）

use std::fmt;

/// 对应 CmpDefinitionException：组件定义错误（v2.16.0 新增）
#[derive(Debug, Clone)]
pub struct CmpDefinitionException {
    /// 异常信息
    pub message: String,
}

impl CmpDefinitionException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for CmpDefinitionException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CmpDefinitionException {}
