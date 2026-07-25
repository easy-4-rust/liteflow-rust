//! 对应 Java 类：com.yomahub.liteflow.spi.holder.ContextAwareHolder
//!
//! 环境容器 SPI 工厂类。Java 通过 ServiceLoader 加载全部实现、按
//! priority 升序取首个并缓存为静态单例；Rust 无 ServiceLoader，
//! 以 `OnceLock<RwLock<Option<Arc<dyn ContextAware>>>>` 全局单例替代：
//! - load_context_aware()：未注册时回退 Local 默认实现（对应 list.get(0)
//!   在非 spring 环境即 LocalContextAware）；
//! - register()：显式注册覆盖（对应 ServiceLoader 中更高优先级实现）；
//! - clean()：清空缓存，下次 load 重新回退 Local。

use std::sync::{Arc, OnceLock, RwLock};

use crate::spi::context_aware::ContextAware;
use crate::spi::local::LocalContextAware;

static HOLDER: OnceLock<RwLock<Option<Arc<dyn ContextAware>>>> = OnceLock::new();

fn cell() -> &'static RwLock<Option<Arc<dyn ContextAware>>> {
    HOLDER.get_or_init(|| RwLock::new(None))
}

/// 对应 ContextAwareHolder
pub struct ContextAwareHolder;

impl ContextAwareHolder {
    /// 对应 loadContextAware()：未注册时回退 LocalContextAware
    pub fn load_context_aware() -> Arc<dyn ContextAware> {
        if let Some(x) = cell().read().unwrap().as_ref() {
            return x.clone();
        }
        let mut guard = cell().write().unwrap();
        guard
            .get_or_insert_with(|| Arc::new(LocalContextAware::new()))
            .clone()
    }

    /// 显式注册覆盖实现（对应 ServiceLoader 高优先级实现入选）
    pub fn register(context_aware: Arc<dyn ContextAware>) {
        *cell().write().unwrap() = Some(context_aware);
    }

    /// 对应 clean()
    pub fn clean() {
        *cell().write().unwrap() = None;
    }
}
