//! 对应 Java 类：com.yomahub.liteflow.exception.ScriptBeanMethodInvokeException
//!
//! 脚本 Bean 方法调用错误

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 ScriptBeanMethodInvokeException：脚本 Bean 方法调用错误
#[derive(Debug, Clone)]
pub struct ScriptBeanMethodInvokeException {
    /// 节点 ID
    pub node: String,
    /// 异常信息
    pub message: String,
}

impl ScriptBeanMethodInvokeException {
    /// 创建异常（对应 Java 的构造器）
    pub fn new(node: impl Into<String>, message: impl Into<String>) -> Self {
        Self { node: node.into(), message: message.into() }
    }
}

impl fmt::Display for ScriptBeanMethodInvokeException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "script bean method invoke error in node[{}]: {}", self.node, self.message)
    }
}

impl std::error::Error for ScriptBeanMethodInvokeException {}

impl From<ScriptBeanMethodInvokeException> for LiteflowError {
    fn from(e: ScriptBeanMethodInvokeException) -> Self {
        LiteflowError::Script { node: e.node, msg: e.message }
    }
}
