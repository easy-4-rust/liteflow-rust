//! LiteFlow 结果别名。

use super::LiteflowError;

/// LiteFlow 统一结果类型。
pub type LFResult<T> = Result<T, LiteflowError>;
