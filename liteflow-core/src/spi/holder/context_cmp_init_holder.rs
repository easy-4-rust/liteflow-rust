//! 对应 Java 类：com.yomahub.liteflow.spi.holder.ContextCmpInitHolder
//!
//! 环境组件初始化 SPI 工厂类。Holder 模式说明见 context_aware_holder.rs。

use std::sync::{Arc, OnceLock, RwLock};

use crate::spi::context_cmp_init::ContextCmpInit;
use crate::spi::local::LocalContextCmpInit;

static HOLDER: OnceLock<RwLock<Option<Arc<dyn ContextCmpInit>>>> = OnceLock::new();

fn cell() -> &'static RwLock<Option<Arc<dyn ContextCmpInit>>> {
    HOLDER.get_or_init(|| RwLock::new(None))
}

/// 对应 ContextCmpInitHolder
pub struct ContextCmpInitHolder;

impl ContextCmpInitHolder {
    /// 对应 loadContextCmpInit()：未注册时回退 LocalContextCmpInit
    pub fn load_context_cmp_init() -> Arc<dyn ContextCmpInit> {
        if let Some(x) = cell().read().unwrap().as_ref() {
            return x.clone();
        }
        let mut guard = cell().write().unwrap();
        guard
            .get_or_insert_with(|| Arc::new(LocalContextCmpInit::new()))
            .clone()
    }

    /// 显式注册覆盖实现
    pub fn register(context_cmp_init: Arc<dyn ContextCmpInit>) {
        *cell().write().unwrap() = Some(context_cmp_init);
    }

    /// 对应 clean()
    pub fn clean() {
        *cell().write().unwrap() = None;
    }
}
