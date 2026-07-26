//! GraalJS 兼容入口与 Vernal 组合场景。

use liteflow_core::script::ScriptExecutorFactory;
use liteflow_script_graaljs::GraalJavaScriptExecutor;
use liteflow_vernal::LiteflowConfig;

/// 注册隔离的 GraalJS 兼容执行器。
pub async fn run_case() -> bool {
    GraalJavaScriptExecutor::register().is_ok()
        && ScriptExecutorFactory::contains("graaljs")
        && LiteflowConfig::new().enable
}
