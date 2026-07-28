//! 对应 Java 类：`com.yomahub.liteflow.exception.LiteFlowException`。

use std::error::Error;
use std::fmt;
use std::sync::Arc;

pub use super::liteflow_error::LiteflowError;

/// LiteFlow 框架内部逻辑异常基类。
///
/// 独立类型让业务可以像 Java 一样按异常家族识别错误，同时保留可选业务状态码、
/// 原始消息和底层 cause。具体 Java 异常仍由同包独立文件定义，并转换为统一
/// `LiteflowError` 供 Rust `Result` 主干传播。
///
/// 对应 Java: `com.yomahub.liteflow.exception.LiteFlowException`。
#[derive(Debug, Clone)]
pub struct LiteFlowException {
    code: Option<String>,
    message: String,
    cause: Option<Arc<dyn Error + Send + Sync>>,
}

impl LiteFlowException {
    /// 使用异常描述创建异常。
    ///
    /// - `message`: Java 参数 `message`，异常描述信息。
    /// - 返回：不含状态码和 cause 的异常。
    ///
    /// 对应 Java: `LiteFlowException#LiteFlowException(String)`。
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            code: None,
            message: message.into(),
            cause: None,
        }
    }

    /// 使用业务状态码和异常描述创建异常。
    ///
    /// - `code`: Java 参数 `code`，异常状态码。
    /// - `message`: Java 参数 `message`，异常描述信息。
    /// - 返回：包含状态码的异常。
    ///
    /// 对应 Java: `LiteFlowException#LiteFlowException(String, String)`。
    #[must_use]
    pub fn with_code(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: Some(code.into()),
            message: message.into(),
            cause: None,
        }
    }

    /// 使用底层异常创建异常。
    ///
    /// - `cause`: Java 参数 `cause`，底层异常对象。
    /// - 返回：消息取自 cause 的异常。
    ///
    /// 对应 Java: `LiteFlowException#LiteFlowException(Throwable)`。
    #[must_use]
    pub fn from_cause(cause: impl Error + Send + Sync + 'static) -> Self {
        let message = cause.to_string();
        Self {
            code: None,
            message,
            cause: Some(Arc::new(cause)),
        }
    }

    /// 使用描述和底层异常创建异常。
    ///
    /// - `message`: Java 参数 `message`，对外异常信息。
    /// - `cause`: Java 参数 `cause`，底层异常对象。
    /// - 返回：包含 cause 的异常。
    ///
    /// 对应 Java: `LiteFlowException#LiteFlowException(String, Throwable)`。
    #[must_use]
    pub fn with_cause(
        message: impl Into<String>,
        cause: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            code: None,
            message: message.into(),
            cause: Some(Arc::new(cause)),
        }
    }

    /// 使用状态码、描述和底层异常创建异常。
    ///
    /// - `code`: Java 参数 `code`，异常状态码。
    /// - `message`: Java 参数 `message`，对外异常信息。
    /// - `cause`: Java 参数 `cause`，底层异常对象。
    /// - 返回：保留全部 Java 基类字段的异常。
    ///
    /// 对应 Java: `LiteFlowException#LiteFlowException(String, String, Throwable)`。
    #[must_use]
    pub fn with_code_and_cause(
        code: impl Into<String>,
        message: impl Into<String>,
        cause: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            code: Some(code.into()),
            message: message.into(),
            cause: Some(Arc::new(cause)),
        }
    }

    /// 返回异常状态码。
    ///
    /// - 返回：未设置时为 `None`，对应 Java `null`。
    ///
    /// 对应 Java: `LiteFlowException#getCode`。
    #[must_use]
    pub fn get_code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    /// 返回原始异常消息。
    ///
    /// - 返回：构造时保存的消息。
    ///
    /// 对应 Java: `Throwable#getMessage`。
    #[must_use]
    pub fn get_message(&self) -> &str {
        &self.message
    }

    /// 返回底层异常。
    ///
    /// - 返回：未设置 cause 时为 `None`。
    ///
    /// 对应 Java: `Throwable#getCause`。
    #[must_use]
    pub fn get_cause(&self) -> Option<&(dyn Error + Send + Sync + 'static)> {
        self.cause.as_deref()
    }
}

impl fmt::Display for LiteFlowException {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for LiteFlowException {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause
            .as_deref()
            .map(|cause| cause as &(dyn Error + 'static))
    }
}
