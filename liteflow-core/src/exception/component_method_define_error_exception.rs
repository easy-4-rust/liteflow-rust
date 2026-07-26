//! 对应 Java 类：com.yomahub.liteflow.exception.ComponentMethodDefineErrorException
//!
//! 组件方法定义错误（声明了非法的组件方法）

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 ComponentMethodDefineErrorException：组件方法定义错误（声明了非法的组件方法）
#[derive(Debug, Clone)]
pub struct ComponentMethodDefineErrorException {
    /// 异常信息
    pub message: String,
}

impl ComponentMethodDefineErrorException {
    /// 创建异常（对应 Java 的 message 构造器）
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ComponentMethodDefineErrorException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ComponentMethodDefineErrorException {}

impl From<ComponentMethodDefineErrorException> for LiteflowError {
    fn from(e: ComponentMethodDefineErrorException) -> Self {
        LiteflowError::CmpDefine(e.message)
    }
}
