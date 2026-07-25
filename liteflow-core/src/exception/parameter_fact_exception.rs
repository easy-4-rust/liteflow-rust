//! 对应 Java 类：com.yomahub.liteflow.exception.ParameterFactException
//!
//! 参数构造错误（v2.16.0 新增）

use std::fmt;

/// 对应 ParameterFactException：参数构造错误（v2.16.0 新增）
#[derive(Debug, Clone)]
pub struct ParameterFactException {
    /// 异常信息
    pub message: String,
}

impl ParameterFactException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for ParameterFactException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParameterFactException {}
