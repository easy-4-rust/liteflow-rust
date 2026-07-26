//! 循环 EL 包装器使用的函数类别。

/// FOR、WHILE、ITERATOR 三类循环函数。
///
/// 对应 Java: `LoopELWrapper` 内部根据构造入口选择的 EL 函数名。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoopFunction {
    For,
    While,
    Iterator,
}

impl LoopFunction {
    /// 返回 LiteFlow EL 函数名。
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::For => "FOR",
            Self::While => "WHILE",
            Self::Iterator => "ITERATOR",
        }
    }
}
