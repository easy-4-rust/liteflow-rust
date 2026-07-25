//! 对应 Java 类：com.yomahub.liteflow.spi.local.LocalContextCmpInit
//!
//! 非 Spring 环境容器上下文组件初始化实现：空实现。
//! Spring 体系下基于扫描的组件初始化实现器由后续 vernal（spring 等价层）
//! 阶段实现。

use crate::spi::context_cmp_init::ContextCmpInit;
use crate::spi::spi_priority::SpiPriority;

/// 对应 LocalContextCmpInit
#[derive(Default)]
pub struct LocalContextCmpInit;

impl LocalContextCmpInit {
    pub fn new() -> Self {
        Self
    }
}

impl ContextCmpInit for LocalContextCmpInit {
    /// 对应 initCmp()：非 spring 环境不用实现
    fn init_cmp(&self) {}
}

impl SpiPriority for LocalContextCmpInit {
    /// 对应 priority()
    fn priority(&self) -> i32 {
        2
    }
}
