//! 对应 aop 包 + spi.CmpAroundAspect：全局组件切面。

use crate::slot::CmpContext;
use async_trait::async_trait;
use std::sync::Arc;

/// 对应 ICmpAroundAspect / CmpAroundAspect SPI
#[async_trait]
pub trait CmpAroundAspect: Send + Sync + 'static {
    /// 组件执行前（对应 beforeProcess 之前的全局切面）
    async fn before(&self, _ctx: &CmpContext) {}
    /// 组件执行后（无论成败）
    async fn after(&self, _ctx: &CmpContext) {}
    /// 组件异常时（对齐 onError 时机的切面扩展）
    async fn on_error(&self, _ctx: &CmpContext, _e: &crate::exception::LiteflowError) {}
}

/// 切面注册表（对应 CmpAroundAspectHolder）
#[derive(Clone, Default)]
pub struct AspectHolder {
    aspects: Arc<Vec<Arc<dyn CmpAroundAspect>>>,
}

impl AspectHolder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn register(&mut self, aspect: Arc<dyn CmpAroundAspect>) {
        Arc::get_mut(&mut self.aspects)
            .expect("aspect holder already shared")
            .push(aspect);
    }
    pub fn aspects(&self) -> &[Arc<dyn CmpAroundAspect>] {
        &self.aspects
    }
    pub fn is_empty(&self) -> bool {
        self.aspects.is_empty()
    }
}
