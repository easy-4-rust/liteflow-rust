//! 内建 Rhai 脚本与 Vernal 组合场景。

use liteflow_core::script::RhaiScriptExecutor;
use liteflow_vernal::LiteflowConfig;

/// 编译真实 Rhai 表达式并校验 Vernal 配置。
pub async fn run_case() -> bool {
    RhaiScriptExecutor::new().validate("40 + 2") && LiteflowConfig::new().enable
}
