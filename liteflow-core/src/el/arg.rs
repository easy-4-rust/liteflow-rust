//! EL 操作符调用参数。

use super::El;

/// EL Operator 的类型化调用参数。
///
/// Java `BaseOperator#call(Object...)` 在运行期接收任意对象；Rust 将允许的
/// 表达式、字符串、数字、布尔与空值显式建模，避免向下转型失败。
/// 这是 Rust 专用 AST 伴随类型，不对应独立 Java 对象。
#[derive(Debug, Clone)]
pub enum Arg {
    /// 已经构建完成的 EL 表达式。
    Expr(El),
    /// 字符串参数，通常表示节点 ID、标签或绑定值。
    Str(String),
    /// 数字参数，用于次数、比例和超时等 Operator。
    Num(f64),
    /// 布尔参数，用于 ANY、MUST 与 WHILE 字面量。
    Bool(bool),
    /// QLExpress 的 `null` 参数；由 OperatorHelper 转换为 DataNotFoundException 语义。
    Null,
}
