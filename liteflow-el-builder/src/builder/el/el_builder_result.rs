//! EL 构建结果别名。

use super::ELBuilderError;

/// EL 构建结果。
pub type ELBuilderResult<T> = Result<T, ELBuilderError>;
