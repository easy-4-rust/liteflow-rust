//! 对应 Java 类：com.yomahub.liteflow.spi.holder.SpiFactoryCleaner
//!
//! 统一清理全部 SPI Holder 缓存（Java 源码位于 spi/holder 包下；
//! Rust 侧按迁移约定置于 spi 包根，语义一致）。

use super::holder::SpiFactoryInitializing;

/// 对应 SpiFactoryCleaner
pub struct SpiFactoryCleaner;

impl SpiFactoryCleaner {
    /// 对应 clean()：清空全部 holder，下次 load 回退各 Local 默认实现
    pub fn clean() {
        SpiFactoryInitializing::clean();
    }
}
