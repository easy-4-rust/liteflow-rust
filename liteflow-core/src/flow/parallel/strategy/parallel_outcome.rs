//! 并行分支结算结果。

use std::collections::HashSet;

use crate::exception::LiteflowError;

/// 并行分支统一结算结果。
///
/// 这是 Rust 并行策略执行器共享的内部拥有型结果，汇总 Java
/// ParallelStrategyExecutor 各实现中的局部集合与异常，不对应独立 Java 对象。
pub struct ParallelOutcome {
    /// 已完成分支序号；成功、失败和超时都属于完成。
    pub completed: HashSet<usize>,
    /// 成功分支序号。
    pub oks: HashSet<usize>,
    /// 已完成且真正超时的分支序号与 ID。
    pub timeout_items: Vec<(usize, String)>,
    /// 首个错误。
    pub first_err: Option<LiteflowError>,
    /// 首个错误在原始分支列表中的序号，用于保持 Java 列表遍历顺序。
    pub(crate) first_err_index: Option<usize>,
    /// 是否有分支触发 ChainEnd。
    pub chain_end: bool,
    /// 指定序号分支的错误。
    pub must_err: Option<LiteflowError>,
    /// 指定分支首错的原始序号。
    pub(crate) must_err_index: Option<usize>,
}
