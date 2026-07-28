use std::sync::{Arc, RwLock};

use liteflow_core::aop::ICmpAroundAspect;

/// 保存 Vernal 上下文扫描到的全局组件切面。
///
/// Rust 将 Holder 作为应用上下文单例注册，避免 Java 静态字段在多个上下文间
/// 相互覆盖。对应 Java:
/// `com.yomahub.liteflow.spring.process.holder.SpringCmpAroundAspectHolder`。
#[derive(Default)]
pub struct SpringCmpAroundAspectHolder {
    instance: RwLock<Option<Arc<dyn ICmpAroundAspect>>>,
}

impl SpringCmpAroundAspectHolder {
    /// 创建未初始化的切面持有器。
    ///
    /// # 返回
    /// 当前应用上下文独享的空持有器。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 初始化业务切面。
    ///
    /// # 参数
    /// - `instance`：容器扫描到的真实 `ICmpAroundAspect` 实例。
    ///
    /// 对应 Java: `SpringCmpAroundAspectHolder#init`。
    pub fn init(&self, instance: Arc<dyn ICmpAroundAspect>) {
        *self
            .instance
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(instance);
    }

    /// 返回当前业务切面。
    ///
    /// # 返回
    /// 已初始化切面的共享快照；未发现切面时返回 `None`。对应 Java:
    /// `SpringCmpAroundAspectHolder#getInstance`。
    #[must_use]
    pub fn get_instance(&self) -> Option<Arc<dyn ICmpAroundAspect>> {
        self.instance
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// 清理当前上下文保存的业务切面。
    ///
    /// 对应 Java: `SpringCmpAroundAspectHolder#clean`。
    pub fn clean(&self) {
        *self
            .instance
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    }
}
