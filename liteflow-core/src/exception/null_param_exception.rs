//! 对应 Java 类：com.yomahub.liteflow.exception.NullParamException
//!
//! 参数为空

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 NullParamException：参数为空
#[derive(Debug, Clone)]
pub struct NullParamException {
    /// 异常信息
    pub message: String,
}

impl NullParamException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for NullParamException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NullParamException {}

impl From<NullParamException> for LiteflowError {
    fn from(e: NullParamException) -> Self {
        LiteflowError::NullParam(e.message)
    }
}
