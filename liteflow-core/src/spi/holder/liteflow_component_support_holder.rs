//! 对应 Java 类：com.yomahub.liteflow.spi.holder.LiteflowComponentSupportHolder
//!
//! liteflowComponent 支持扩展 SPI 工厂类。Holder 模式说明见
//! context_aware_holder.rs。

use std::sync::{Arc, OnceLock, RwLock};

use crate::spi::liteflow_component_support::LiteflowComponentSupport;
use crate::spi::local::LocalLiteflowComponentSupport;

static HOLDER: OnceLock<RwLock<Option<Arc<dyn LiteflowComponentSupport>>>> = OnceLock::new();

fn cell() -> &'static RwLock<Option<Arc<dyn LiteflowComponentSupport>>> {
    HOLDER.get_or_init(|| RwLock::new(None))
}

/// 对应 LiteflowComponentSupportHolder
pub struct LiteflowComponentSupportHolder;

impl LiteflowComponentSupportHolder {
    /// 对应 loadLiteflowComponentSupport()：未注册时回退 LocalLiteflowComponentSupport
    pub fn load_liteflow_component_support() -> Arc<dyn LiteflowComponentSupport> {
        if let Some(x) = cell().read().unwrap().as_ref() {
            return x.clone();
        }
        let mut guard = cell().write().unwrap();
        guard
            .get_or_insert_with(|| Arc::new(LocalLiteflowComponentSupport::new()))
            .clone()
    }

    /// 显式注册覆盖实现
    pub fn register(support: Arc<dyn LiteflowComponentSupport>) {
        *cell().write().unwrap() = Some(support);
    }

    /// 对应 clean()
    pub fn clean() {
        *cell().write().unwrap() = None;
    }
}
