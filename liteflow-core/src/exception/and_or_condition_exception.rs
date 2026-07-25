//! 对应 Java 类：com.yomahub.liteflow.exception.AndOrConditionException
//!
//! AND/OR 条件错误（v2.16.0 新增）

use std::fmt;

/// 对应 AndOrConditionException：AND/OR 条件错误（v2.16.0 新增）
#[derive(Debug, Clone)]
pub struct AndOrConditionException {
    /// 异常信息
    pub message: String,
}

impl AndOrConditionException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for AndOrConditionException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AndOrConditionException {}
