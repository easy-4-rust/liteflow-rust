//! Aviator 脚本与 Vernal 组合场景。

use liteflow_core::script::ScriptExecutorFactory;
use liteflow_script_aviator::AviatorScriptExecutor;
use liteflow_vernal::LiteflowConfig;

/// 注册 Aviator 公共表达式子集执行器。
pub async fn run_case() -> bool {
    AviatorScriptExecutor::register().is_ok()
        && ScriptExecutorFactory::contains("aviator")
        && LiteflowConfig::new().enable
}
