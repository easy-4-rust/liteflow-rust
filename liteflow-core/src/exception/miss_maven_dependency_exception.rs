//! 对应 Java 类：com.yomahub.liteflow.exception.MissMavenDependencyException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 缺少 Maven 依赖异常。
#[derive(Debug, Clone)]
pub struct MissMavenDependencyException {
    message: String,
}

impl MissMavenDependencyException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for MissMavenDependencyException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for MissMavenDependencyException {}

impl LiteFlowException for MissMavenDependencyException {
    fn message(&self) -> &str {
        &self.message
    }
}
