//! 对应 Java: `com.yomahub.liteflow.script.aviator.AviatorScriptExecutor`。

use std::sync::Arc;

use liteflow_core::LFResult;
use liteflow_core::core::NodeComponent;
use liteflow_core::script::{ScriptExecutorFactory, ScriptKind, build_rhai_component};

/// Aviator 公共表达式子集执行器。
pub struct AviatorScriptExecutor;

impl AviatorScriptExecutor {
    /// 注册 `language = "aviator"`。
    pub fn register() -> LFResult<()> {
        ScriptExecutorFactory::register("aviator", Self::build)
    }

    fn build(node_id: &str, kind: ScriptKind, script: &str) -> LFResult<Arc<dyn NodeComponent>> {
        build_rhai_component(node_id, kind, normalize_expression(script))
    }
}

/// 将 Aviator 与 Rhai 的公共表达式子集规范化。
fn normalize_expression(script: &str) -> &str {
    let script = script.trim();
    let script = script.strip_suffix(';').unwrap_or(script).trim();
    script.strip_prefix("return ").unwrap_or(script).trim()
}
