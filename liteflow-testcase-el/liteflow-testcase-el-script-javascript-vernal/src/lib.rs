//! JavaScript 脚本与 Vernal 组合场景。

use liteflow_core::script::ScriptExecutorFactory;
use liteflow_script_javascript::JavaScriptExecutor;
use liteflow_vernal::LiteflowConfig;

/// 注册 Boa JavaScript 执行器。
pub async fn run_case() -> bool {
    JavaScriptExecutor::register().is_ok()
        && ScriptExecutorFactory::contains("javascript")
        && LiteflowConfig::new().enable
}
