//! 对应 script 包 + liteflow-script-plugin 的 Rust 化实现。
//! Java 版每种脚本语言一个插件模块（JSR223）；Rust 版内建 rhai 引擎，
//! 其余语言（groovy/js/lua/python/kotlin/qlexpress/aviator）为 Java 生态特有，
//! 语义替代方案见迁移对照表。

pub mod script_executor;
pub mod script_component;
pub mod json_convert;
#[cfg(feature = "lua")]
pub mod lua_executor;

pub use script_component::{ScriptComponent, ScriptKind};
