//! EL 词法 token。

/// 递归下降解析器内部 token。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Tok {
    Ident(String),
    Str(String),
    Num(f64),
    Bool(bool),
    LP,
    RP,
    Comma,
    Dot,
}
