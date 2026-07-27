//! Apollo 规则插件异常。

use std::fmt::{Display, Formatter};

use liteflow_core::exception::LiteflowError;

/// 保存 Apollo 初始化、读取和转换过程中的错误消息。
///
/// 对应 Java: `com.yomahub.liteflow.parser.apollo.exception.ApolloException`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApolloException {
    message: String,
}

impl ApolloException {
    /// 使用错误消息创建异常。对应 Java `ApolloException#ApolloException(String)`。
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// 返回原始异常消息。对应 Java `ApolloException#getMessage`。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for ApolloException {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ApolloException {}

impl From<ApolloException> for LiteflowError {
    fn from(error: ApolloException) -> Self {
        Self::Rule(error.to_string())
    }
}
