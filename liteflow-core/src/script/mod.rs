//! 对应 core/script 包。
//! core 内建 Rhai 基线与插件工厂；Lua/JavaScript/Python 等语言实现位于独立
//! `liteflow-script-plugin` crate，通过 `ScriptExecutorFactory` 显式注册。

pub mod exception;
pub mod json_convert;
pub mod jsr223;
pub mod proxy;
mod rhai_script_component_factory;
mod rhai_script_executor;
pub mod script_bean_manager;
mod script_component_builder;
pub mod script_execute_wrap;
pub mod script_executor;
mod script_executor_component;
pub mod script_executor_factory;
mod script_iterator_component;
mod script_kind;
pub mod validator;

pub use crate::core::{
    ScriptBooleanComponent, ScriptCommonComponent, ScriptComponent, ScriptForComponent,
    ScriptSwitchComponent,
};
pub use rhai_script_component_factory::build_rhai_component;
pub use rhai_script_executor::RhaiScriptExecutor;
pub use script_bean_manager::ScriptBeanManager;
pub use script_component_builder::ScriptComponentBuilder;
pub use script_execute_wrap::ScriptExecuteWrap;
pub use script_executor::ScriptExecutor;
pub use script_executor_component::ScriptExecutorComponent;
pub use script_executor_factory::ScriptExecutorFactory;
pub use script_iterator_component::ScriptIteratorComponent;
pub use script_kind::ScriptKind;
