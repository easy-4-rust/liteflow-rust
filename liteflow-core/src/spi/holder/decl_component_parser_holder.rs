//! Java `DeclComponentParserHolder` 的 Rust 映射。

use std::sync::{Arc, OnceLock, RwLock};

use crate::spi::DeclComponentParser;
use crate::spi::local::LocalDeclComponentParser;

static HOLDER: OnceLock<RwLock<Option<Arc<dyn DeclComponentParser>>>> = OnceLock::new();

fn cell() -> &'static RwLock<Option<Arc<dyn DeclComponentParser>>> {
    HOLDER.get_or_init(|| RwLock::new(None))
}

/// 声明式组件解析 SPI 工厂。
///
/// Java 使用 `ServiceLoader` 按 priority 选取实现；Rust 使用显式注册，未注册时
/// 回退 `LocalDeclComponentParser`。
///
/// 对应 Java: `com.yomahub.liteflow.spi.holder.DeclComponentParserHolder`。
pub struct DeclComponentParserHolder;

impl DeclComponentParserHolder {
    /// 返回当前声明式组件解析器。
    ///
    /// 对应 Java: `DeclComponentParserHolder#loadDeclComponentParser`。
    pub fn load_decl_component_parser() -> Arc<dyn DeclComponentParser> {
        if let Some(parser) = cell().read().expect("声明式解析器读锁中毒").as_ref() {
            return parser.clone();
        }
        let mut guard = cell().write().expect("声明式解析器写锁中毒");
        guard
            .get_or_insert_with(|| Arc::new(LocalDeclComponentParser::new()))
            .clone()
    }

    /// 显式注册容器提供的声明式组件解析器。
    pub fn register(decl_component_parser: Arc<dyn DeclComponentParser>) {
        *cell().write().expect("声明式解析器写锁中毒") = Some(decl_component_parser);
    }

    /// 清空解析器缓存。对应 Java: `DeclComponentParserHolder#clean`。
    pub fn clean() {
        *cell().write().expect("声明式解析器写锁中毒") = None;
    }
}
