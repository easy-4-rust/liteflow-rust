//! 对应 Java 类：com.yomahub.liteflow.spi.holder.PathContentParserHolder
//!
//! 路径内容解析 SPI 工厂类。Holder 模式说明见 context_aware_holder.rs。

use std::sync::{Arc, OnceLock, RwLock};

use crate::spi::local::LocalPathContentParser;
use crate::spi::path_content_parser::PathContentParser;

static HOLDER: OnceLock<RwLock<Option<Arc<dyn PathContentParser>>>> = OnceLock::new();

fn cell() -> &'static RwLock<Option<Arc<dyn PathContentParser>>> {
    HOLDER.get_or_init(|| RwLock::new(None))
}

/// 对应 PathContentParserHolder
pub struct PathContentParserHolder;

impl PathContentParserHolder {
    /// 加载当前路径内容解析器。
    ///
    /// Java 原方法名为 `loadContextAware`，但实际返回 `PathContentParser`；未显式注册
    /// 实现时返回线程安全缓存的 `LocalPathContentParser`。
    /// 对应 Java: `PathContentParserHolder#loadContextAware`。
    pub fn load_context_aware() -> Arc<dyn PathContentParser> {
        if let Some(x) = cell().read().unwrap().as_ref() {
            return x.clone();
        }
        let mut guard = cell().write().unwrap();
        guard
            .get_or_insert_with(|| Arc::new(LocalPathContentParser::new()))
            .clone()
    }

    /// 加载当前路径内容解析器。
    ///
    /// 这是 Rust 侧修正 Java 误命名后的可读入口，行为与
    /// `load_context_aware` 完全一致。
    #[must_use]
    pub fn load_path_content_parser() -> Arc<dyn PathContentParser> {
        Self::load_context_aware()
    }

    /// 显式注册覆盖实现
    pub fn register(path_content_parser: Arc<dyn PathContentParser>) {
        *cell().write().unwrap() = Some(path_content_parser);
    }

    /// 对应 clean()
    pub fn clean() {
        *cell().write().unwrap() = None;
    }
}
