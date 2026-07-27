//! 对应 Java 类：com.yomahub.liteflow.spi.local.LocalCmpAroundAspect
//!
//! 非 Spring 环境全局组件切面实现：空实现
//! （无 spring 环境下不支持全局组件切面）。

use crate::exception::LiteflowError;
use crate::slot::Slot;
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
    /// 本地环境没有容器切面，按 Java Local 实现保持无副作用。参数 `node_id`
    /// 和 `slot` 用于与容器实现保持相同协议。对应 Java:
    /// `LocalCmpAroundAspect#beforeProcess`。
    pub fn before_process(&self, node_id: &str, slot: &Slot) {
        let _ = (node_id, slot);
    }

    /// 在组件 finally 阶段处理。
    ///
    /// 本地环境按 Java Local 实现保持无副作用。对应 Java:
    /// `LocalCmpAroundAspect#afterProcess`。
    pub fn after_process(&self, node_id: &str, slot: &Slot) {
        let _ = (node_id, slot);
    }

    /// 接收组件成功事件。
    ///
    /// 本地环境按 Java Local 实现保持无副作用。对应 Java:
    /// `LocalCmpAroundAspect#onSuccess`。
    pub fn on_success(&self, node_id: &str, slot: &Slot) {
        let _ = (node_id, slot);
    }

    /// 接收组件失败事件。
    ///
    /// 本地环境按 Java Local 实现保持无副作用，原始错误继续由执行主干传播。
    /// 对应 Java: `LocalCmpAroundAspect#onError`。
    pub fn on_error(&self, node_id: &str, slot: &Slot, error: &LiteflowError) {
        let _ = (node_id, slot, error);
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
    fn before_process(&self, node_id: &str, slot: &Slot) {
        Self::before_process(self, node_id, slot);
    }

    /// 对应 afterProcess：无 spring 环境下为空实现
    fn after_process(&self, node_id: &str, slot: &Slot) {
        Self::after_process(self, node_id, slot);
    }

    fn on_success(&self, node_id: &str, slot: &Slot) {
        Self::on_success(self, node_id, slot);
    }

    fn on_error(&self, node_id: &str, slot: &Slot, error: &LiteflowError) {
        Self::on_error(self, node_id, slot, error);
    }
}

impl SpiPriority for LocalCmpAroundAspect {
    /// 对应 priority()
    fn priority(&self) -> i32 {
        Self::priority(self)
    }
}
