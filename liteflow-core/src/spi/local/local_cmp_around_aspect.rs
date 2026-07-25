//! 对应 Java 类：com.yomahub.liteflow.spi.local.LocalCmpAroundAspect
//!
//! 非 Spring 环境全局组件切面实现：空实现
//! （无 spring 环境下不支持全局组件切面）。

use crate::slot::Slot;
use crate::spi::cmp_around_aspect::CmpAroundAspect;
use crate::spi::spi_priority::SpiPriority;

/// 对应 LocalCmpAroundAspect
#[derive(Default)]
pub struct LocalCmpAroundAspect;

impl LocalCmpAroundAspect {
    pub fn new() -> Self {
        Self
    }
}

impl CmpAroundAspect for LocalCmpAroundAspect {
    /// 对应 beforeProcess：无 spring 环境下为空实现
    fn before_process(&self, _node_id: &str, _slot: &Slot) {}

    /// 对应 afterProcess：无 spring 环境下为空实现
    fn after_process(&self, _node_id: &str, _slot: &Slot) {}
}

impl SpiPriority for LocalCmpAroundAspect {
    /// 对应 priority()
    fn priority(&self) -> i32 {
        2
    }
}
