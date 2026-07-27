use std::error::Error;

/// Agent 模型调用、工具执行或最终回复处理失败。
///
/// 对应 Java: `com.yomahub.liteflow.agent.exception.AgentInvocationException`。
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct AgentInvocationException {
    message: String,
    #[source]
    source: Option<Box<dyn Error + Send + Sync>>,
}

impl AgentInvocationException {
    /// 使用调用错误消息创建异常。
    ///
    /// 对应 Java: `AgentInvocationException#AgentInvocationException(String)`。
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    /// 使用调用错误消息和底层错误创建异常。
    ///
    /// 对应 Java: `AgentInvocationException#AgentInvocationException(String, Throwable)`。
    #[must_use]
    pub fn with_source(
        message: impl Into<String>,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// 返回不包含底层错误格式化内容的原始调用错误消息。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}
