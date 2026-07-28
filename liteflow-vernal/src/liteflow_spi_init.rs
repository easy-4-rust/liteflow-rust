//! 对应 Java: com.yomahub.liteflow.spring.LiteflowSpiInit

use std::sync::atomic::{AtomicBool, Ordering};

use liteflow_core::spi::SpiFactoryInitializing;

/// LiteFlow 容器 SPI 初始化完成回调。
///
/// 全部 Vernal 单例和优先级为 1 的容器 SPI 注册完毕后，统一触发各 Holder
/// 预加载，避免工作线程第一次访问时因加载时机或宿主类加载边界产生不同实现。
///
/// 对应 Java: `com.yomahub.liteflow.spring.LiteflowSpiInit`。
#[derive(Debug, Default)]
pub struct LiteflowSpiInit {
    initialized: AtomicBool,
}

impl LiteflowSpiInit {
    /// 创建尚未执行预加载的 SPI 初始化回调。
    ///
    /// # 返回
    /// 可作为 Vernal 真实单例注册的回调对象。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 在所有容器单例完成装配后预加载 LiteFlow SPI。
    ///
    /// Holder 会在当前线程确认 ContextAware、组件初始化、声明解析、路径解析、
    /// 组件名称与切面实现，随后发布初始化完成状态。对应 Java:
    /// `LiteflowSpiInit#afterSingletonsInstantiated`。
    pub fn after_singletons_instantiated(&self) {
        SpiFactoryInitializing::load_init();
        self.initialized.store(true, Ordering::Release);
    }

    /// 返回 SPI 预加载是否已经完成。
    ///
    /// # 返回
    /// `after_singletons_instantiated` 成功走完后返回 `true`。这是 Rust 容器
    /// 可观测性扩展，不改变 Java 初始化语义。
    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Acquire)
    }
}
