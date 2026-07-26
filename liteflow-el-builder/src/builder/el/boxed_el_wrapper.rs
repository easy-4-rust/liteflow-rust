//! 盒装 EL 包装器。

use super::ELWrapper;

/// 承载 Java `ELWrapper` 子类运行时多态的 trait object。
pub type BoxedELWrapper = Box<dyn ELWrapper>;
