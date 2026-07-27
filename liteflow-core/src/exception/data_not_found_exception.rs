//! 对应 Java 类：com.yomahub.liteflow.exception.DataNotFoundException
//!
//! 数据未找到

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// Java 无参构造器使用的默认异常信息。
pub const MSG: &str = "DataNotFoundException";

/// 对应 DataNotFoundException：数据未找到
#[derive(Debug, Clone)]
pub struct DataNotFoundException {
    /// 异常信息
    pub message: String,
}

impl DataNotFoundException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// 返回异常信息。
    ///
    /// # 返回
    /// 当前异常持有的原始消息。对应 Java: `DataNotFoundException#getMessage`。
    #[must_use]
    pub fn get_message(&self) -> &str {
        &self.message
    }

    /// 修改异常信息。
    ///
    /// 参数 `message` 对应 Java 同名参数。对应 Java:
    /// `DataNotFoundException#setMessage`。
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}

impl Default for DataNotFoundException {
    /// 使用 Java 常量 `MSG` 创建未找到数据异常。
    fn default() -> Self {
        Self::new(MSG)
    }
}

impl fmt::Display for DataNotFoundException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for DataNotFoundException {}

impl From<DataNotFoundException> for LiteflowError {
    fn from(e: DataNotFoundException) -> Self {
        LiteflowError::DataNotFound(e.message)
    }
}
