//! 对应 Java: `com.yomahub.liteflow.script.groovy.GroovyScriptExecutor`。

use std::sync::Arc;

use liteflow_core::LFResult;
use liteflow_core::core::NodeComponent;
use liteflow_core::script::{ScriptExecutorFactory, ScriptKind, build_rhai_component};

/// Groovy 生态位的 Rust 原生表达式执行器。
pub struct GroovyScriptExecutor;

impl GroovyScriptExecutor {
    /// 注册 `language = "groovy"`。
    pub fn register() -> LFResult<()> {
        ScriptExecutorFactory::register("groovy", Self::build)
    }

    fn build(node_id: &str, kind: ScriptKind, script: &str) -> LFResult<Arc<dyn NodeComponent>> {
        build_rhai_component(node_id, kind, normalize_expression(script))
    }
}

/// 将 Groovy 与 Rhai 的公共表达式子集规范化。
fn normalize_expression(script: &str) -> &str {
    let script = script.trim();
    let script = script.strip_suffix(';').unwrap_or(script).trim();
    script.strip_prefix("return ").unwrap_or(script).trim()
}
