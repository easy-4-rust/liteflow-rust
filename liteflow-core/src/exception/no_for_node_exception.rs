//! 对应 Java 类：com.yomahub.liteflow.exception.NoForNodeException
//!
//! FOR 条件中缺少 FOR 节点

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 NoForNodeException：FOR 条件中缺少 FOR 节点
#[derive(Debug, Clone)]
pub struct NoForNodeException {
    /// 异常信息
    pub message: String,
}

impl NoForNodeException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// 返回异常信息。
    ///
    /// # 返回
    /// 当前异常持有的原始消息。对应 Java: `NoForNodeException#getMessage`。
    #[must_use]
    pub fn get_message(&self) -> &str {
        &self.message
    }

    /// 修改异常信息。
    ///
    /// 参数 `message` 对应 Java 同名参数。对应 Java:
    /// `NoForNodeException#setMessage`。
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}

impl fmt::Display for NoForNodeException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NoForNodeException {}

impl From<NoForNodeException> for LiteflowError {
    fn from(e: NoForNodeException) -> Self {
        LiteflowError::NoForNode(e.message)
    }
}
