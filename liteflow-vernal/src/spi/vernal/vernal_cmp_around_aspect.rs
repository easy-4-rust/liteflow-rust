//! 对应 Java 类：com.yomahub.liteflow.spi.spring.SpringCmpAroundAspect

use std::sync::{Arc, RwLock};

use liteflow_core::aop::ICmpAroundAspect;
use liteflow_core::exception::LiteflowError;
use liteflow_core::slot::CmpContext;
use liteflow_core::spi::{CmpAroundAspect, SpiPriority};

/// Vernal 环境的全局组件切面 SPI 实现。
///
/// Java 实现从 `SpringCmpAroundAspectHolder` 取得容器扫描到的
/// `ICmpAroundAspect`，存在时逐个委托四个生命周期方法。Rust 将同一实例保存
/// 在本对象的线程安全槽位中，并由 `CmpAroundAspectHolder` 装载；没有配置切面
/// 时保持无副作用。
///
/// 对应 Java: `com.yomahub.liteflow.spi.spring.SpringCmpAroundAspect`。
#[derive(Default)]
pub struct VernalCmpAroundAspect {
    instance: RwLock<Option<Arc<dyn ICmpAroundAspect>>>,
}

impl VernalCmpAroundAspect {
    /// 创建 Vernal 全局组件切面适配器。
    ///
    /// # 参数
    /// - `instance`：Vernal 容器托管的业务切面；`None` 表示未配置。
    ///
    /// # 返回
    /// 可注册到 LiteFlow SPI Holder 的适配器。对应 Java:
    /// `SpringCmpAroundAspect#SpringCmpAroundAspect` 与
    /// `SpringCmpAroundAspectHolder#init`。
    #[must_use]
    pub fn new(instance: Option<Arc<dyn ICmpAroundAspect>>) -> Self {
        Self {
            instance: RwLock::new(instance),
        }
    }

    /// 替换当前容器切面实例。
    ///
    /// # 参数
    /// - `instance`：新的业务切面共享实例。
    ///
    /// 对应 Java: `SpringCmpAroundAspectHolder#init`。
    pub fn init(&self, instance: Arc<dyn ICmpAroundAspect>) {
        *self
            .instance
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(instance);
    }

    /// 清除当前容器切面实例。
    ///
    /// 清理后四个回调均保持无副作用。对应 Java:
    /// `SpringCmpAroundAspectHolder#clean`。
    pub fn clean(&self) {
        *self
            .instance
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    }

    /// 返回当前业务切面的共享快照。
    ///
    /// # 返回
    /// 已配置切面，或在未配置时返回 `None`。对应 Java:
    /// `SpringCmpAroundAspectHolder#getInstance`。
    #[must_use]
    pub fn get_instance(&self) -> Option<Arc<dyn ICmpAroundAspect>> {
        self.instance
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// 在组件处理前委托容器切面。
    ///
    /// # 参数
    /// - `context`：当前组件执行上下文。
    ///
    /// 对应 Java: `SpringCmpAroundAspect#beforeProcess`。
    pub fn before_process(&self, context: &CmpContext) {
        if let Some(instance) = self.get_instance() {
            instance.before_process(context);
        }
    }

    /// 在组件 finally 阶段委托容器切面。
    ///
    /// # 参数
    /// - `context`：当前组件执行上下文。
    ///
    /// 对应 Java: `SpringCmpAroundAspect#afterProcess`。
    pub fn after_process(&self, context: &CmpContext) {
        if let Some(instance) = self.get_instance() {
            instance.after_process(context);
        }
    }

    /// 在组件成功后委托容器切面。
    ///
    /// # 参数
    /// - `context`：当前组件执行上下文。
    ///
    /// 对应 Java: `SpringCmpAroundAspect#onSuccess`。
    pub fn on_success(&self, context: &CmpContext) {
        if let Some(instance) = self.get_instance() {
            instance.on_success(context);
        }
    }

    /// 在组件失败后委托容器切面。
    ///
    /// # 参数
    /// - `context`：当前组件执行上下文；
    /// - `error`：组件原始执行错误。
    ///
    /// 对应 Java: `SpringCmpAroundAspect#onError`。
    pub fn on_error(&self, context: &CmpContext, error: &LiteflowError) {
        if let Some(instance) = self.get_instance() {
            instance.on_error(context, error);
        }
    }

    /// 返回容器实现的 SPI 优先级。
    ///
    /// # 返回
    /// 固定返回 `1`，优先于本地空实现。对应 Java:
    /// `SpringCmpAroundAspect#priority`。
    #[must_use]
    pub fn priority(&self) -> i32 {
        1
    }
}

impl CmpAroundAspect for VernalCmpAroundAspect {
    fn before_process(&self, context: &CmpContext) {
        VernalCmpAroundAspect::before_process(self, context);
    }

    fn after_process(&self, context: &CmpContext) {
        VernalCmpAroundAspect::after_process(self, context);
    }

    fn on_success(&self, context: &CmpContext) {
        VernalCmpAroundAspect::on_success(self, context);
    }

    fn on_error(&self, context: &CmpContext, error: &LiteflowError) {
        VernalCmpAroundAspect::on_error(self, context, error);
    }
}

impl ICmpAroundAspect for VernalCmpAroundAspect {
    fn before_process(&self, context: &CmpContext) {
        VernalCmpAroundAspect::before_process(self, context);
    }

    fn after_process(&self, context: &CmpContext) {
        VernalCmpAroundAspect::after_process(self, context);
    }

    fn on_success(&self, context: &CmpContext) {
        VernalCmpAroundAspect::on_success(self, context);
    }

    fn on_error(&self, context: &CmpContext, error: &LiteflowError) {
        VernalCmpAroundAspect::on_error(self, context, error);
    }
}

impl SpiPriority for VernalCmpAroundAspect {
    fn priority(&self) -> i32 {
        VernalCmpAroundAspect::priority(self)
    }
}
