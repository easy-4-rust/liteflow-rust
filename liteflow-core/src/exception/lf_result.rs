//! LiteFlow 结果别名。

use super::LiteflowError;

/// LiteFlow 统一结果类型。
///
/// 这是 Rust 对 Java checked/unchecked exception 传播的 Result 映射，不对应
/// 独立 Java 对象；错误分支统一使用 LiteflowError。
pub type LFResult<T> = Result<T, LiteflowError>;
