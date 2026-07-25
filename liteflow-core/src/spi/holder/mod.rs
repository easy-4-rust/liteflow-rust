//! 对应 Java 包：com.yomahub.liteflow.spi.holder
//!
//! 各 SPI 的 Holder 工厂类：Java ServiceLoader 单例在 Rust 侧以
//! `OnceLock<RwLock<Option<Arc<dyn Xxx>>>>` 全局单例实现。

pub mod context_aware_holder;
pub mod cmp_around_aspect_holder;
pub mod context_cmp_init_holder;
pub mod liteflow_component_support_holder;
pub mod path_content_parser_holder;

pub use context_aware_holder::ContextAwareHolder;
pub use cmp_around_aspect_holder::CmpAroundAspectHolder;
pub use context_cmp_init_holder::ContextCmpInitHolder;
pub use liteflow_component_support_holder::LiteflowComponentSupportHolder;
pub use path_content_parser_holder::PathContentParserHolder;
