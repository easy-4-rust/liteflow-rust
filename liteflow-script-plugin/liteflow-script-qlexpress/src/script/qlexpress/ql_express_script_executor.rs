//! 对应 Java: `com.yomahub.liteflow.script.qlexpress.QLExpressScriptExecutor`。

use std::sync::Arc;

use liteflow_core::LFResult;
use liteflow_core::core::NodeComponent;
use liteflow_core::script::{ScriptExecutorFactory, ScriptKind, build_rhai_component};

/// QLExpress 公共表达式子集执行器。
pub struct QlExpressScriptExecutor;

impl QlExpressScriptExecutor {
    /// 注册 `language = "qlexpress"`。
    pub fn register() -> LFResult<()> {
        ScriptExecutorFactory::register("qlexpress", Self::build)
    }

    fn build(node_id: &str, kind: ScriptKind, script: &str) -> LFResult<Arc<dyn NodeComponent>> {
        build_rhai_component(node_id, kind, normalize_expression(script))
    }
}

/// 将 QLExpress 与 Rhai 的公共表达式子集规范化。
fn normalize_expression(script: &str) -> &str {
    let script = script.trim();
    let script = script.strip_suffix(';').unwrap_or(script).trim();
    script.strip_prefix("return ").unwrap_or(script).trim()
}
