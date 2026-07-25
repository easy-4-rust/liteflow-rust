//! 对应 Java 类：com.yomahub.liteflow.exception.CatchErrorException
//!
//! 组件执行捕获到错误时抛出（CATCH 语义载体）

use std::fmt;

/// 对应 CatchErrorException：组件执行捕获到错误时抛出（CATCH 语义载体）
#[derive(Debug, Clone)]
pub struct CatchErrorException {
    /// 异常信息
    pub message: String,
}

impl CatchErrorException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for CatchErrorException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CatchErrorException {}
