//! 对应 Java 类：com.yomahub.liteflow.exception.NoMatchedRouteChainException
//!
//! 无匹配的路由链（v2.16.0 新增）

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 NoMatchedRouteChainException：无匹配的路由链（v2.16.0 新增）
#[derive(Debug, Clone)]
pub struct NoMatchedRouteChainException {
    /// 异常信息
    pub message: String,
}

impl NoMatchedRouteChainException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for NoMatchedRouteChainException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NoMatchedRouteChainException {}

impl From<NoMatchedRouteChainException> for LiteflowError {
    fn from(_e: NoMatchedRouteChainException) -> Self {
        LiteflowError::NoMatchedRouteChain
    }
}
