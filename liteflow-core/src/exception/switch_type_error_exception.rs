//! 对应 Java 类：com.yomahub.liteflow.exception.SwitchTypeErrorException
//!
//! SWITCH 节点返回类型错误（应返回字符串目标）

use std::fmt;

use super::lite_flow_exception::LiteflowError;

/// 对应 SwitchTypeErrorException：SWITCH 节点返回类型错误（应返回字符串目标）
#[derive(Debug, Clone)]
pub struct SwitchTypeErrorException {
    /// 节点 ID
    pub node: String,
    /// 期望返回类型
    pub expect: String,
    /// 实际返回类型
    pub actual: String,
}

impl SwitchTypeErrorException {
    /// 创建异常（对应 Java 的构造器）
    pub fn new(
        node: impl Into<String>,
        expect: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        Self {
            node: node.into(),
            expect: expect.into(),
            actual: actual.into(),
        }
    }
}

impl fmt::Display for SwitchTypeErrorException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "node[{}] should return {}, but got {}",
            self.node, self.expect, self.actual
        )
    }
}

impl std::error::Error for SwitchTypeErrorException {}

impl From<SwitchTypeErrorException> for LiteflowError {
    fn from(e: SwitchTypeErrorException) -> Self {
        LiteflowError::NodeTypeError {
            node: e.node,
            expect: e.expect,
            actual: e.actual,
        }
    }
}
