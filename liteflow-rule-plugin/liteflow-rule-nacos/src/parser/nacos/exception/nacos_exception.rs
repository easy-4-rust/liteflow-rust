//! Nacos 规则插件异常。

use std::fmt::{Display, Formatter};

use liteflow_core::exception::LiteflowError;

/// 保存 Nacos 初始化、读取、监听与内容校验错误。
///
/// 对应 Java: `com.yomahub.liteflow.parser.nacos.exception.NacosException`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NacosException {
    message: String,
}

impl NacosException {
    /// 使用错误消息创建异常。对应 Java `NacosException#NacosException(String)`。
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// 返回原始异常消息。对应 Java `NacosException#getMessage`。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for NacosException {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for NacosException {}

impl From<nacos_sdk::api::error::Error> for NacosException {
    fn from(error: nacos_sdk::api::error::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<NacosException> for LiteflowError {
    fn from(error: NacosException) -> Self {
        Self::Rule(error.to_string())
    }
}
