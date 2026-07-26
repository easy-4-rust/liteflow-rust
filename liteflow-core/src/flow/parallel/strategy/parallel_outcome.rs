//! 并行分支结算结果。

use std::collections::HashSet;

use crate::exception::LiteflowError;

/// 并行分支统一结算结果。
pub struct ParallelOutcome {
    /// 成功分支序号。
    pub oks: HashSet<usize>,
    /// 首个错误。
    pub first_err: Option<LiteflowError>,
    /// 是否有分支触发 ChainEnd。
    pub chain_end: bool,
    /// 指定序号分支的错误。
    pub must_err: Option<LiteflowError>,
}
