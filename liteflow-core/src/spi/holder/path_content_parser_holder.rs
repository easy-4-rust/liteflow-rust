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
    /// 对应 loadContextAware()（Java 原方法名如此，实为加载 PathContentParser）：
    /// 未注册时回退 LocalPathContentParser
    pub fn load_path_content_parser() -> Arc<dyn PathContentParser> {
        if let Some(x) = cell().read().unwrap().as_ref() {
            return x.clone();
        }
        let mut guard = cell().write().unwrap();
        guard
            .get_or_insert_with(|| Arc::new(LocalPathContentParser::new()))
            .clone()
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
