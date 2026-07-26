//! 循环 EL 包装器的数据来源。

use super::BoxedELWrapper;

/// 固定次数或动态表达式两种循环来源。
///
/// 对应 Java: `LoopELWrapper` 的 `object` 字段在数字与 ELWrapper 间的联合语义。
pub(crate) enum LoopSource {
    Number(u32),
    Expression(BoxedELWrapper),
}
