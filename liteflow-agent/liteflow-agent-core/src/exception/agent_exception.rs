use std::error::Error;

/// LiteFlow Agent 模块所有领域错误的基础错误对象。
///
/// Rust 不使用运行时异常继承；该对象保留 Java 基类的消息与可选底层错误链，
/// 具体配置/调用错误使用各自独立类型，并在框架边界转换为 `AgentError`。
///
/// 对应 Java: `com.yomahub.liteflow.agent.exception.AgentException`。
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct AgentException {
    message: String,
    #[source]
    source: Option<Box<dyn Error + Send + Sync>>,
}

impl AgentException {
    /// 使用错误消息创建 Agent 基础错误。
    ///
    /// 对应 Java: `AgentException#AgentException(String)`。
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    /// 使用错误消息和底层错误创建 Agent 基础错误。
    ///
    /// 对应 Java: `AgentException#AgentException(String, Throwable)`。
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

    /// 返回不包含底层错误格式化内容的原始消息。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}
