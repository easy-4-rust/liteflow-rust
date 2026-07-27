use std::error::Error;

/// Agent 配置缺失、取值无效或装配失败。
///
/// 对应 Java: `com.yomahub.liteflow.agent.exception.AgentConfigException`。
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct AgentConfigException {
    message: String,
    #[source]
    source: Option<Box<dyn Error + Send + Sync>>,
}

impl AgentConfigException {
    /// 使用配置错误消息创建异常。
    ///
    /// 对应 Java: `AgentConfigException#AgentConfigException(String)`。
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    /// 使用配置错误消息和底层错误创建异常。
    ///
    /// 对应 Java: `AgentConfigException#AgentConfigException(String, Throwable)`。
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

    /// 返回不包含底层错误格式化内容的原始配置错误消息。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}
