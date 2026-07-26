//! 对应 Java: com.yomahub.liteflow.script.exception.ScriptLoadException

use std::fmt;

use crate::exception::LiteflowError;

/// 脚本加载、编译或缓存查找失败。
#[derive(Debug, Clone, Default)]
pub struct ScriptLoadException {
    node_id: String,
    message: String,
}

impl ScriptLoadException {
    /// 创建脚本加载异常。
    #[must_use]
    pub fn new(node_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
            message: message.into(),
        }
    }
    /// 返回异常消息。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
    /// 修改异常消息，对应 Java 可变 message 字段。
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
        Self::Script {
            node: error.node_id,
            msg: error.message,
        }
    }
}
