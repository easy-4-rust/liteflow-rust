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
    ///
    /// # 返回
    /// 当前异常持有的原始消息。对应 Java: `ScriptSpiException#getMessage`。
    #[must_use]
    pub fn get_message(&self) -> &str {
        &self.message
    }

    /// 返回异常消息。
    ///
    /// 这是既有 Rust API，委托 Java 命名入口读取同一字段。
    #[must_use]
    pub fn message(&self) -> &str {
        self.get_message()
    }

    /// 修改异常消息。
    ///
    /// 参数 `message` 对应 Java 同名参数。对应 Java:
    /// `ScriptSpiException#setMessage`。
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
        Self::Custom(error.message)
    }
}
