//! 对应 Java 类：com.yomahub.liteflow.spi.local.LocalContextCmpInit
//!
//! 非 Spring 环境容器上下文组件初始化实现：空实现。
//! Spring 体系下基于扫描的组件初始化实现器由后续 vernal（spring 等价层）
//! 阶段实现。

use crate::spi::context_cmp_init::ContextCmpInit;
use crate::spi::spi_priority::SpiPriority;

/// 非容器环境的组件初始化 SPI。
///
/// 本地模式中的组件由调用方直接构造并注册，因此初始化动作按 Java 实现保持
/// 无副作用，优先级固定为 `2`。对应 Java:
/// `com.yomahub.liteflow.spi.local.LocalContextCmpInit`。
#[derive(Default)]
pub struct LocalContextCmpInit;

impl LocalContextCmpInit {
    /// 创建本地组件初始化器。
    ///
    /// 对应 Java: `LocalContextCmpInit#LocalContextCmpInit`。
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// 执行非容器环境的组件初始化。
    ///
    /// 本地组件在注册前已经完成构造，因此该方法不扫描容器，也不修改组件。
    /// 对应 Java: `LocalContextCmpInit#initCmp`。
    pub fn init_cmp(&self) {
        // 保留本地模式的显式初始化边界；Vernal 实现会在同一 SPI 位置扫描组件。
        let _ = self;
    }

    /// 返回本地实现的 SPI 优先级。
    ///
    /// - 返回：固定值 `2`，数值越小优先级越高。
    ///
    /// 对应 Java: `LocalContextCmpInit#priority`。
    #[must_use]
    pub fn priority(&self) -> i32 {
        2
    }
}

impl ContextCmpInit for LocalContextCmpInit {
    /// 对应 initCmp()：非 spring 环境不用实现
    fn init_cmp(&self) {
        Self::init_cmp(self);
    }
}

impl SpiPriority for LocalContextCmpInit {
    /// 对应 priority()
    fn priority(&self) -> i32 {
        Self::priority(self)
    }
}
