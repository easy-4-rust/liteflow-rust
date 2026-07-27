//! LiteFlow 的 Vernal 容器与 Web 框架集成层。
//!
//! 对应 Java `liteflow-spring`、`liteflow-spring-boot-starter` 与 Quarkus
//! 扩展的职责，但使用 Vernal 显式模块、Axum 和 Actix Web 实现。

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
mod liteflow_vernal_config;
mod liteflow_vernal_error;
mod liteflow_vernal_module;
mod rule_initialization_state;
mod shared_registration;

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
pub use liteflow_vernal_config::LiteflowVernalConfig;
pub use liteflow_vernal_config::LiteflowVernalConfig as LiteflowConfig;
pub use liteflow_vernal_error::LiteflowVernalError;
pub use liteflow_vernal_module::LiteflowVernalModule;
pub(crate) use shared_registration::SharedRegistration;
