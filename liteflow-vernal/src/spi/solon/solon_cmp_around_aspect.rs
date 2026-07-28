use std::sync::{Arc, RwLock};

use liteflow_core::aop::ICmpAroundAspect;
use liteflow_core::exception::LiteflowError;
use liteflow_core::slot::CmpContext;
use liteflow_core::spi::{CmpAroundAspect, SpiPriority};

/// Solon 环境的全局组件切面 SPI 实现。
///
/// Java 构造器通过 `Solon.context().getBeanAsync` 延迟取得
/// `ICmpAroundAspect`；Rust 以可更新的线程安全槽位保存同一共享切面，支持插件
/// 启动前后注册而不复制业务对象。对应 Java:
/// `com.yomahub.liteflow.spi.solon.SolonCmpAroundAspect`。
#[derive(Default)]
pub struct SolonCmpAroundAspect {
    cmp_around_aspect: RwLock<Option<Arc<dyn ICmpAroundAspect>>>,
}

impl SolonCmpAroundAspect {
    /// 创建 Solon 全局切面适配器。
    ///
    /// # 参数
    /// - `cmp_around_aspect`：当前容器切面；`None` 表示尚未异步发现。
    #[must_use]
    pub fn new(cmp_around_aspect: Option<Arc<dyn ICmpAroundAspect>>) -> Self {
        Self {
            cmp_around_aspect: RwLock::new(cmp_around_aspect),
        }
    }

    /// 注入容器发现的全局切面。
    ///
    /// # 参数
    /// - `cmp_around_aspect`：Solon 托管的真实切面实例。
    ///
    /// 对应 Java 构造器中的 `getBeanAsync` 回调。
    pub fn set_cmp_around_aspect(&self, cmp_around_aspect: Arc<dyn ICmpAroundAspect>) {
        *self
            .cmp_around_aspect
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(cmp_around_aspect);
    }

    /// 返回当前容器切面共享快照。
    ///
    /// # 返回
    /// 已发现的切面，或 `None`。
    #[must_use]
    pub fn get_cmp_around_aspect(&self) -> Option<Arc<dyn ICmpAroundAspect>> {
        self.cmp_around_aspect
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// 在组件处理前委托全局切面。
    ///
    /// # 参数
    /// - `context`：当前节点执行上下文。对应 Java: `beforeProcess`。
    pub fn before_process(&self, context: &CmpContext) {
        if let Some(cmp_around_aspect) = self.get_cmp_around_aspect() {
            cmp_around_aspect.before_process(context);
        }
    }

    /// 在组件 finally 阶段委托全局切面。
    ///
    /// # 参数
    /// - `context`：当前节点执行上下文。对应 Java: `afterProcess`。
    pub fn after_process(&self, context: &CmpContext) {
        if let Some(cmp_around_aspect) = self.get_cmp_around_aspect() {
            cmp_around_aspect.after_process(context);
        }
    }

    /// 在组件成功后委托全局切面。
    ///
    /// # 参数
    /// - `context`：当前节点执行上下文。对应 Java: `onSuccess`。
    pub fn on_success(&self, context: &CmpContext) {
        if let Some(cmp_around_aspect) = self.get_cmp_around_aspect() {
            cmp_around_aspect.on_success(context);
        }
    }

    /// 在组件失败后委托全局切面。
    ///
    /// # 参数
    /// - `context`：当前节点执行上下文；
    /// - `error`：原始执行错误。对应 Java: `onError`。
    pub fn on_error(&self, context: &CmpContext, error: &LiteflowError) {
        if let Some(cmp_around_aspect) = self.get_cmp_around_aspect() {
            cmp_around_aspect.on_error(context, error);
        }
    }

    /// 返回 Solon SPI 优先级。
    ///
    /// # 返回
    /// 固定为 `1`。对应 Java: `priority`。
    #[must_use]
    pub fn priority(&self) -> i32 {
        1
    }
}

impl CmpAroundAspect for SolonCmpAroundAspect {
    fn before_process(&self, context: &CmpContext) {
        SolonCmpAroundAspect::before_process(self, context);
    }

    fn after_process(&self, context: &CmpContext) {
        SolonCmpAroundAspect::after_process(self, context);
    }

    fn on_success(&self, context: &CmpContext) {
        SolonCmpAroundAspect::on_success(self, context);
    }

    fn on_error(&self, context: &CmpContext, error: &LiteflowError) {
        SolonCmpAroundAspect::on_error(self, context, error);
    }
}

impl ICmpAroundAspect for SolonCmpAroundAspect {
    fn before_process(&self, context: &CmpContext) {
        SolonCmpAroundAspect::before_process(self, context);
    }

    fn after_process(&self, context: &CmpContext) {
        SolonCmpAroundAspect::after_process(self, context);
    }

    fn on_success(&self, context: &CmpContext) {
        SolonCmpAroundAspect::on_success(self, context);
    }

    fn on_error(&self, context: &CmpContext, error: &LiteflowError) {
        SolonCmpAroundAspect::on_error(self, context, error);
    }
}

impl SpiPriority for SolonCmpAroundAspect {
    fn priority(&self) -> i32 {
        SolonCmpAroundAspect::priority(self)
    }
}
