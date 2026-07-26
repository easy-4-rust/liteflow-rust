//! Python 脚本与 Vernal 组合场景。

use liteflow_core::script::ScriptExecutorFactory;
use liteflow_script_python::PythonScriptExecutor;
use liteflow_vernal::LiteflowConfig;

/// 注册 PyO3 CPython 执行器。
pub async fn run_case() -> bool {
    PythonScriptExecutor::register().is_ok()
        && ScriptExecutorFactory::contains("python")
        && LiteflowConfig::new().enable
}
