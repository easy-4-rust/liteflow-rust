//! 对应 Java 包：com.yomahub.liteflow.spi
//!
//! SPI 体系：5 个 SPI 接口（均继承 SpiPriority）+ holder 子包
//! （Java ServiceLoader 单例的 Rust 化）+ local 子包（非 Spring 默认实现）
//! + SpiFactoryCleaner。

mod bean;
pub mod cmp_around_aspect;
pub mod context_aware;
pub mod context_cmp_init;
pub mod holder;
pub mod liteflow_component_support;
pub mod local;
pub mod path_content_parser;
pub mod spi_factory_cleaner;
pub mod spi_priority;

pub use bean::Bean;
pub use cmp_around_aspect::CmpAroundAspect;
pub use context_aware::ContextAware;
pub use context_cmp_init::ContextCmpInit;
pub use holder::{
    CmpAroundAspectHolder, ContextAwareHolder, ContextCmpInitHolder,
    LiteflowComponentSupportHolder, PathContentParserHolder,
};
pub use liteflow_component_support::LiteflowComponentSupport;
pub use local::{
    LocalCmpAroundAspect, LocalContextAware, LocalContextCmpInit, LocalLiteflowComponentSupport,
    LocalPathContentParser,
};
pub use path_content_parser::PathContentParser;
pub use spi_factory_cleaner::SpiFactoryCleaner;
pub use spi_priority::SpiPriority;
