//! EL 包装器类别。

/// Java 运行时用 `instanceof` 区分的包装器类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapperKind {
    /// 普通节点。
    CommonNode,
    /// AND/OR/NOT 布尔运算表达式。
    BooleanOperator,
    /// 不能作为布尔运算参数的控制流表达式。
    NonBoolean,
}

impl WrapperKind {
    pub(crate) fn is_boolean_capable(self) -> bool {
        matches!(self, Self::CommonNode | Self::BooleanOperator)
    }

    pub(crate) fn is_boolean_operator(self) -> bool {
        matches!(self, Self::BooleanOperator)
    }
}
