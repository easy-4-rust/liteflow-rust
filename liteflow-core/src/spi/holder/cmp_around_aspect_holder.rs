//! 对应 Java 类：com.yomahub.liteflow.spi.holder.CmpAroundAspectHolder
//!
//! 组件全局拦截器 SPI 工厂类。Holder 模式说明见 context_aware_holder.rs。

use std::sync::{Arc, OnceLock, RwLock};

use crate::spi::cmp_around_aspect::CmpAroundAspect;
use crate::spi::local::LocalCmpAroundAspect;

static HOLDER: OnceLock<RwLock<Option<Arc<dyn CmpAroundAspect>>>> = OnceLock::new();

fn cell() -> &'static RwLock<Option<Arc<dyn CmpAroundAspect>>> {
    HOLDER.get_or_init(|| RwLock::new(None))
}

/// 对应 CmpAroundAspectHolder
pub struct CmpAroundAspectHolder;

impl CmpAroundAspectHolder {
    /// 对应 loadCmpAroundAspect()：未注册时回退 LocalCmpAroundAspect
    pub fn load_cmp_around_aspect() -> Arc<dyn CmpAroundAspect> {
        if let Some(x) = cell().read().unwrap().as_ref() {
            return x.clone();
        }
        let mut guard = cell().write().unwrap();
        guard
            .get_or_insert_with(|| Arc::new(LocalCmpAroundAspect::new()))
            .clone()
    }

    /// 显式注册覆盖实现
    pub fn register(cmp_around_aspect: Arc<dyn CmpAroundAspect>) {
        *cell().write().unwrap() = Some(cmp_around_aspect);
    }

    /// 对应 clean()
    pub fn clean() {
        *cell().write().unwrap() = None;
    }
}
