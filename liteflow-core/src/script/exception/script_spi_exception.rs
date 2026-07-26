//! 对应 Java: com.yomahub.liteflow.script.exception.ScriptSpiException

use std::fmt;

use crate::exception::LiteflowError;

/// 脚本语言 SPI 未注册或初始化失败。
#[derive(Debug, Clone, Default)]
pub struct ScriptSpiException {
    message: String,
}

impl ScriptSpiException {
    /// 创建脚本 SPI 异常。
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
    /// 返回异常消息。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
    /// 修改异常消息。
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}

impl fmt::Display for ScriptSpiException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::error::Error for ScriptSpiException {}
impl From<ScriptSpiException> for LiteflowError {
    fn from(error: ScriptSpiException) -> Self {
        Self::Script {
            node: String::new(),
            msg: error.message,
        }
    }
}
