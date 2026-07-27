//! SQL 插件业务异常。

use std::fmt::{Display, Formatter};

use liteflow_core::exception::LiteflowError;

/// 封装 SQL 配置、连接、查询与结果映射错误。
///
/// 对应 Java: `com.yomahub.liteflow.parser.sql.exception.ELSQLException`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ELSQLException {
    message: String,
}

impl ELSQLException {
    /// 使用异常信息创建 SQL 业务异常。对应 Java `ELSQLException#ELSQLException(String)`。
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// 返回异常信息。对应 Java `ELSQLException#getMessage`。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// 修改异常信息。对应 Java `ELSQLException#setMessage`。
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}

impl Display for ELSQLException {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ELSQLException {}

impl From<rusqlite::Error> for ELSQLException {
    fn from(error: rusqlite::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<ELSQLException> for LiteflowError {
    fn from(error: ELSQLException) -> Self {
        LiteflowError::Rule(error.to_string())
    }
}
