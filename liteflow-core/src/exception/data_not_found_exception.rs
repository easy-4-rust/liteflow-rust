//! 对应 Java 类：com.yomahub.liteflow.exception.DataNotFoundException
//!
//! 数据未找到

use std::fmt;

/// 对应 DataNotFoundException：数据未找到
#[derive(Debug, Clone)]
pub struct DataNotFoundException {
    /// 异常信息
    pub message: String,
}

impl DataNotFoundException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for DataNotFoundException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for DataNotFoundException {}
