//! 对应 Java 类：com.yomahub.liteflow.exception.ChainEndException
//!
//! 链终止信号（正常终止，不算执行失败）

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 ChainEndException：链终止信号（正常终止，不算执行失败）
#[derive(Debug, Clone)]
pub struct ChainEndException {
    /// 异常信息
    pub message: String,
}

impl ChainEndException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ChainEndException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ChainEndException {}

impl From<ChainEndException> for LiteflowError {
    fn from(_e: ChainEndException) -> Self {
        LiteflowError::ChainEnd
    }
}
