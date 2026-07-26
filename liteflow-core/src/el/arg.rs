//! EL 操作符调用参数。

use super::El;

/// 方法调用参数。
#[derive(Debug, Clone)]
pub(crate) enum Arg {
    Expr(El),
    Str(String),
    Num(f64),
    Bool(bool),
}
