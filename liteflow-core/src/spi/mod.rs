//! 对应 Java 包：com.yomahub.liteflow.spi
//!
//! SPI 体系：5 个 SPI 接口（均继承 SpiPriority）+ holder 子包
//! （Java ServiceLoader 单例的 Rust 化）+ local 子包（非 Spring 默认实现）
//! + SpiFactoryCleaner。

pub mod spi_priority;
pub mod context_aware;
pub mod cmp_around_aspect;
pub mod context_cmp_init;
pub mod liteflow_component_support;
pub mod path_content_parser;
pub mod holder;
pub mod local;
pub mod spi_factory_cleaner;

pub use spi_priority::SpiPriority;
pub use context_aware::{Bean, ContextAware};
pub use cmp_around_aspect::CmpAroundAspect;
pub use context_cmp_init::ContextCmpInit;
pub use liteflow_component_support::LiteflowComponentSupport;
pub use path_content_parser::PathContentParser;
pub use holder::{
    CmpAroundAspectHolder, ContextAwareHolder, ContextCmpInitHolder,
    LiteflowComponentSupportHolder, PathContentParserHolder,
};
pub use local::{
    LocalCmpAroundAspect, LocalContextAware, LocalContextCmpInit,
    LocalLiteflowComponentSupport, LocalPathContentParser,
};
pub use spi_factory_cleaner::SpiFactoryCleaner;
