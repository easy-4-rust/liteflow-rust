//! LiteFlow 的 Vernal 容器与 Web 框架集成层。
//!
//! 对应 Java `liteflow-spring`、Spring Boot 3/4 starter 的职责，使用
//! Vernal 显式模块与 Axum 实现；Actix 是 Rust Web 补充适配。Java v2.16.0
//! 基线没有 Quarkus 生产模块，因此不虚构 Quarkus 对象映射。

#[cfg(feature = "actix")]
mod liteflow_actix_service;
#[cfg(feature = "axum")]
mod liteflow_axum_router;
mod liteflow_component_registration;
mod liteflow_execute_request;
mod liteflow_execute_response;
mod liteflow_parse_mode;
mod liteflow_rule_format;
mod liteflow_runtime;
mod liteflow_spi_init;
mod liteflow_vernal_config;
mod liteflow_vernal_error;
mod liteflow_vernal_module;
pub mod process;
mod rule_initialization_state;
mod shared_registration;
pub mod solon;
pub mod spi;
pub mod springboot;
pub mod springboot4;
mod vernal_component_scanner;
mod vernal_decl_bean_definition;

#[cfg(feature = "actix")]
pub use liteflow_actix_service::LiteflowActixService;
#[cfg(feature = "axum")]
pub use liteflow_axum_router::LiteflowAxumRouter;
pub use liteflow_component_registration::LiteflowComponentRegistration;
pub use liteflow_core::LiteflowConfigGetter;
pub use liteflow_execute_request::LiteflowExecuteRequest;
pub use liteflow_execute_response::LiteflowExecuteResponse;
pub use liteflow_parse_mode::LiteflowParseMode;
pub use liteflow_rule_format::LiteflowRuleFormat;
pub use liteflow_runtime::LiteflowRuntime;
pub use liteflow_spi_init::LiteflowSpiInit;
pub use liteflow_vernal_config::LiteflowVernalConfig;
pub use liteflow_vernal_config::LiteflowVernalConfig as LiteflowConfig;
pub use liteflow_vernal_error::LiteflowVernalError;
pub use liteflow_vernal_module::LiteflowVernalModule;
pub(crate) use shared_registration::SharedRegistration;
pub use spi::{
    VernalAware, VernalCmpAroundAspect, VernalContextCmpInit, VernalDeclComponentParser,
    VernalLiteflowComponentSupport, VernalPathContentParser,
};
pub use springboot::config::{LiteflowMainAutoConfiguration, LiteflowPropertyAutoConfiguration};
pub use springboot::{
    LiteflowExecutorInit, LiteflowMonitorProperty, LiteflowProperty,
    LiteflowPropertyChainCacheProperty,
};
pub use vernal_component_scanner::VernalComponentScanner;
pub use vernal_decl_bean_definition::VernalDeclBeanDefinition;
