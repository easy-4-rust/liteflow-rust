//! LiteFlow 脚本语言插件。
//!
//! 对应 Java `liteflow-script-plugin` 聚合模块。每种脚本语言位于独立子
//! crate；本 crate 仅负责 feature 编排和统一注册。

#[cfg(feature = "aviator")]
pub use liteflow_script_aviator as aviator;
#[cfg(feature = "graaljs")]
pub use liteflow_script_graaljs as graaljs;
#[cfg(feature = "groovy")]
pub use liteflow_script_groovy as groovy;
#[cfg(feature = "javascript")]
pub use liteflow_script_javascript as javascript;
#[cfg(feature = "kotlin")]
pub use liteflow_script_kotlin as kotlin;
#[cfg(feature = "lua")]
pub use liteflow_script_lua as lua;
#[cfg(feature = "python")]
pub use liteflow_script_python as python;
#[cfg(feature = "qlexpress")]
pub use liteflow_script_qlexpress as qlexpress;

use liteflow_core::LFResult;

/// 注册当前启用 feature 对应的全部脚本执行器。
pub fn register_all() -> LFResult<()> {
    #[cfg(feature = "lua")]
    lua::LuaScriptExecutor::register()?;
    #[cfg(feature = "javascript")]
    javascript::JavaScriptExecutor::register()?;
    #[cfg(feature = "kotlin")]
    kotlin::KotlinScriptExecutor::register()?;
    #[cfg(feature = "python")]
    python::PythonScriptExecutor::register()?;
    #[cfg(feature = "groovy")]
    groovy::GroovyScriptExecutor::register()?;
    #[cfg(feature = "qlexpress")]
    qlexpress::QlExpressScriptExecutor::register()?;
    #[cfg(feature = "aviator")]
    aviator::AviatorScriptExecutor::register()?;
    #[cfg(feature = "graaljs")]
    graaljs::GraalJavaScriptExecutor::register()?;
    Ok(())
}
