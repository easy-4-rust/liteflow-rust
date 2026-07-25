//! 对应 Java 类：com.yomahub.liteflow.spi.ContextCmpInit
//!
//! 环境容器中组件初始化 SPI 接口。Java 分 2 个实现：
//! 非 spring 环境下的空实现与 spring 体系下的扫描初始化实现。

use super::spi_priority::SpiPriority;

/// 对应 ContextCmpInit
pub trait ContextCmpInit: SpiPriority + Send + Sync {
    /// 对应 initCmp()
    fn init_cmp(&self);
}
