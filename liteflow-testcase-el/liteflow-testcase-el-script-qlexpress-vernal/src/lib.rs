//! QLExpress 脚本与 Vernal 组合场景。

use liteflow_core::script::ScriptExecutorFactory;
use liteflow_script_qlexpress::QlExpressScriptExecutor;
use liteflow_vernal::LiteflowConfig;

/// 注册 QLExpress 公共表达式子集执行器。
pub async fn run_case() -> bool {
    QlExpressScriptExecutor::register().is_ok()
        && ScriptExecutorFactory::contains("qlexpress")
        && LiteflowConfig::new().enable
}
