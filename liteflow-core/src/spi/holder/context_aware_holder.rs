use std::sync::Arc;

use crate::spi::{SpiFactory, context_aware::ContextAware};

/// 上下文感知持有者
///
/// 用于获取上下文感知实例
pub struct ContextAwareHolder {
    /// 上下文感知实例
    context_aware: Arc<dyn ContextAware>,
}

impl ContextAwareHolder {
    /// 创建上下文感知持有者
    ///
    /// 通过 SPI 工厂获取上下文感知实例
    pub fn new() -> Self {
        let context_aware = SpiFactory::context_aware();
        Self { context_aware }
    }

    /// 获取上下文感知实例
    ///
    /// 返回上下文感知实例
    pub fn context_aware(&self) -> Arc<dyn ContextAware> {
        self.context_aware.clone()
    }
}
