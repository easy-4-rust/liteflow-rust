//! Lua 脚本与 Vernal 组合场景。

use liteflow_core::script::ScriptExecutorFactory;
use liteflow_script_lua::LuaScriptExecutor;
use liteflow_vernal::LiteflowConfig;

/// 注册 mlua 执行器。
pub async fn run_case() -> bool {
    LuaScriptExecutor::register().is_ok()
        && ScriptExecutorFactory::contains("lua")
        && LiteflowConfig::new().enable
}
