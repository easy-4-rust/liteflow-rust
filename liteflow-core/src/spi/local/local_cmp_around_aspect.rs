//! 对应 Java 类：com.yomahub.liteflow.spi.local.LocalCmpAroundAspect
//!
//! 非 Spring 环境全局组件切面实现：空实现
//! （无 spring 环境下不支持全局组件切面）。

use crate::exception::LiteflowError;
use crate::slot::CmpContext;
use crate::spi::cmp_around_aspect::CmpAroundAspect;
use crate::spi::spi_priority::SpiPriority;

/// 对应 LocalCmpAroundAspect
#[derive(Default)]
pub struct LocalCmpAroundAspect;

impl LocalCmpAroundAspect {
    /// 创建非容器环境的空切面实现。
    ///
    /// 对应 Java: `LocalCmpAroundAspect#LocalCmpAroundAspect`。
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// 在组件执行前处理。
    ///
    /// 本地环境没有容器切面，按 Java Local 实现保持无副作用。参数 `context`
    /// 用于与容器实现保持相同协议。对应 Java:
    /// `LocalCmpAroundAspect#beforeProcess`。
    pub fn before_process(&self, context: &CmpContext) {
        let _ = context;
    }

    /// 在组件 finally 阶段处理。
    ///
    /// 本地环境按 Java Local 实现保持无副作用。对应 Java:
    /// `LocalCmpAroundAspect#afterProcess`。
    pub fn after_process(&self, context: &CmpContext) {
        let _ = context;
    }

    /// 接收组件成功事件。
    ///
    /// 本地环境按 Java Local 实现保持无副作用。对应 Java:
    /// `LocalCmpAroundAspect#onSuccess`。
    pub fn on_success(&self, context: &CmpContext) {
        let _ = context;
    }

    /// 接收组件失败事件。
    ///
    /// 本地环境按 Java Local 实现保持无副作用，原始错误继续由执行主干传播。
    /// 对应 Java: `LocalCmpAroundAspect#onError`。
    pub fn on_error(&self, context: &CmpContext, error: &LiteflowError) {
        let _ = (context, error);
    }

    /// 返回 SPI 优先级，数值越小越优先。
    ///
    /// 对应 Java: `LocalCmpAroundAspect#priority`。
    #[must_use]
    pub fn priority(&self) -> i32 {
        2
    }
}

impl CmpAroundAspect for LocalCmpAroundAspect {
    /// 对应 beforeProcess：无 spring 环境下为空实现
    fn before_process(&self, context: &CmpContext) {
        Self::before_process(self, context);
    }

    /// 对应 afterProcess：无 spring 环境下为空实现
    fn after_process(&self, context: &CmpContext) {
        Self::after_process(self, context);
    }

    fn on_success(&self, context: &CmpContext) {
        Self::on_success(self, context);
    }

    fn on_error(&self, context: &CmpContext, error: &LiteflowError) {
        Self::on_error(self, context, error);
    }
}

impl SpiPriority for LocalCmpAroundAspect {
    /// 对应 priority()
    fn priority(&self) -> i32 {
        Self::priority(self)
    }
}
