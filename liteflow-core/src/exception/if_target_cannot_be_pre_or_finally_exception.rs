//! 对应 Java 类：com.yomahub.liteflow.exception.IfTargetCannotBePreOrFinallyException
//!
//! IF 的目标节点不能为 pre/finally 类型

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 IfTargetCannotBePreOrFinallyException：IF 的目标节点不能为 pre/finally 类型
#[derive(Debug, Clone)]
pub struct IfTargetCannotBePreOrFinallyException {
    /// 异常信息
    pub message: String,
}

impl IfTargetCannotBePreOrFinallyException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for IfTargetCannotBePreOrFinallyException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for IfTargetCannotBePreOrFinallyException {}

impl From<IfTargetCannotBePreOrFinallyException> for LiteflowError {
    fn from(e: IfTargetCannotBePreOrFinallyException) -> Self {
        LiteflowError::TargetCannotBePreOrFinally(e.message)
    }
}
