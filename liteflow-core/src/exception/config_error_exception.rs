//! 对应 Java 类：com.yomahub.liteflow.exception.ConfigErrorException

use crate::exception::lite_flow_exception::LiteFlowException;

/// 配置错误异常。
#[derive(Debug, Clone)]
pub struct ConfigErrorException {
    message: String,
    config_key: Option<String>,
}

impl ConfigErrorException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            config_key: None,
        }
    }

    pub fn with_key(message: impl Into<String>, config_key: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            config_key: Some(config_key.into()),
        }
    }

    pub fn config_key(&self) -> Option<&str> {
        self.config_key.as_deref()
    }
}

impl std::fmt::Display for ConfigErrorException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ConfigErrorException {}

impl LiteFlowException for ConfigErrorException {
    fn message(&self) -> &str {
        &self.message
    }
}
