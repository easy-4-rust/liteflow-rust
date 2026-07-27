//! Etcd 解析异常。

use std::fmt::{Display, Formatter};

use liteflow_core::exception::LiteflowError;

/// 保存 Etcd 连接、读取、Watch 与规则转换错误。
///
/// 对应 Java: `com.yomahub.liteflow.parser.etcd.exception.EtcdException`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EtcdException {
    message: String,
}

impl EtcdException {
    /// 使用异常信息创建对象。对应 Java `EtcdException#EtcdException(String)`。
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// 返回异常信息。对应 Java `EtcdException#getMessage`。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// 修改异常信息。对应 Java `EtcdException#setMessage`。
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}

impl Display for EtcdException {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for EtcdException {}

impl From<etcd_client::Error> for EtcdException {
    fn from(error: etcd_client::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<std::str::Utf8Error> for EtcdException {
    fn from(error: std::str::Utf8Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<EtcdException> for LiteflowError {
    fn from(error: EtcdException) -> Self {
        Self::Rule(error.to_string())
    }
}
