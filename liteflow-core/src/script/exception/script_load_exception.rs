//! 对应 Java: com.yomahub.liteflow.script.exception.ScriptLoadException

use std::fmt;

use crate::exception::LiteflowError;

/// 脚本加载、编译或缓存查找失败。
#[derive(Debug, Clone, Default)]
pub struct ScriptLoadException {
    message: String,
}

impl ScriptLoadException {
    /// 使用指定消息创建脚本加载异常。
    ///
    /// 参数 `message` 对应 Java 同名构造参数。对应 Java:
    /// `ScriptLoadException#ScriptLoadException(String)`。
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// 返回异常消息。
    ///
    /// # 返回
    /// 当前异常持有的原始消息。对应 Java: `ScriptLoadException#getMessage`。
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
    /// `ScriptLoadException#setMessage`。
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}

impl fmt::Display for ScriptLoadException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::error::Error for ScriptLoadException {}
impl From<ScriptLoadException> for LiteflowError {
    fn from(error: ScriptLoadException) -> Self {
        // Java 对象只保存 message，转换时不得附加 Rust 自行扩展的节点字段。
        Self::Custom(error.message)
    }
}
