//! ZooKeeper 规则插件异常。

use std::fmt::{Display, Formatter};

use liteflow_core::exception::LiteflowError;

/// 保存 ZooKeeper 连接、读取、监听与规则转换错误。
///
/// 对应 Java: `com.yomahub.liteflow.parser.zk.exception.ZkException`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZkException {
    message: String,
}

impl ZkException {
    /// 使用异常信息创建对象。对应 Java `ZkException#ZkException(String)`。
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// 返回异常信息。对应 Java `ZkException#getMessage`。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// 修改异常信息。对应 Java `ZkException#setMessage`。
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}

impl Display for ZkException {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ZkException {}

impl From<zookeeper_client::Error> for ZkException {
    fn from(error: zookeeper_client::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<std::string::FromUtf8Error> for ZkException {
    fn from(error: std::string::FromUtf8Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<ZkException> for LiteflowError {
    fn from(error: ZkException) -> Self {
        Self::Rule(error.to_string())
    }
}
