//! 对应 Java 类：com.yomahub.liteflow.exception.MissMavenDependencyException
//!
//! 缺少运行所需依赖（v2.16.0 新增）

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 MissMavenDependencyException：缺少运行所需依赖（v2.16.0 新增）
#[derive(Debug, Clone)]
pub struct MissMavenDependencyException {
    /// 异常信息
    pub message: String,
}

impl MissMavenDependencyException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for MissMavenDependencyException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for MissMavenDependencyException {}

impl From<MissMavenDependencyException> for LiteflowError {
    fn from(e: MissMavenDependencyException) -> Self {
        LiteflowError::MissMavenDependency(e.message)
    }
}
