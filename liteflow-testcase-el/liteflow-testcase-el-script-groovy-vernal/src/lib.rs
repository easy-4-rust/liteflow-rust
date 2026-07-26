//! Groovy 脚本与 Vernal 组合场景。

use liteflow_core::script::ScriptExecutorFactory;
use liteflow_script_groovy::GroovyScriptExecutor;
use liteflow_vernal::LiteflowConfig;

/// 注册 Groovy 公共表达式子集执行器。
pub async fn run_case() -> bool {
    GroovyScriptExecutor::register().is_ok()
        && ScriptExecutorFactory::contains("groovy")
        && LiteflowConfig::new().enable
}
