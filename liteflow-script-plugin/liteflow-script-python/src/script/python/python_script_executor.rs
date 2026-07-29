//! 对应 Java: `com.yomahub.liteflow.script.python.PythonScriptExecutor`。
//!
//! 使用 pyo3 嵌入本机 CPython；与 Java Jython 实现一样，把顶层 `return`
//! 转换为 result 变量，并注入 LiteFlow 上下文。

use std::ffi::CString;
use std::sync::Arc;

use async_trait::async_trait;
use liteflow_core::core::NodeComponent;
use liteflow_core::script::{ScriptExecutorFactory, ScriptKind};
use liteflow_core::{CmpContext, LFResult, LiteflowError};
use pyo3::prelude::{PyAnyMethods, PyDictMethods, Python};
use pyo3::types::PyDict;
use serde_json::{Value, json};

/// Python 脚本执行组件。
pub struct PythonScriptExecutor {
    node_id: String,
    kind: ScriptKind,
    script: String,
}

impl PythonScriptExecutor {
    /// 注册 `language = "python"`。
    pub fn register() -> LFResult<()> {
        ScriptExecutorFactory::register("python", Self::build)
    }

    fn build(node_id: &str, kind: ScriptKind, script: &str) -> LFResult<Arc<dyn NodeComponent>> {
        if script.trim().is_empty() {
            return Err(LiteflowError::Script {
                node: node_id.to_string(),
                msg: "python script cannot be blank".to_string(),
            });
        }
        Ok(Arc::new(Self {
            node_id: node_id.to_string(),
            kind,
            script: convert_script(script),
        }))
    }

    /// 执行 CPython，并通过 JSON 边界回收 result/data。
    fn execute(&self, ctx: &CmpContext) -> LFResult<Value> {
        let input = ctx
            .inner
            .input
            .lock()
            .map(|value| value.clone())
            .unwrap_or(Value::Null);
        let data = ctx
            .inner
            .data
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect::<serde_json::Map<_, _>>();
        let runtime = format!(
            r#"
import json
input = json.loads(__lf_input_json)
data = json.loads(__lf_data_json)
node_id = __lf_node_id
tag = json.loads(__lf_tag_json)
loop_index = json.loads(__lf_loop_index_json)
loop_object = json.loads(__lf_loop_object_json)
result = None
{}
__lf_envelope = json.dumps({{"result": result, "data": data}})
"#,
            self.script
        );
        let code = CString::new(runtime)
            .map_err(|error| self.error(format!("python source error: {error}")))?;

        Python::attach(|python| -> LFResult<Value> {
            let locals = PyDict::new(python);
            locals
                .set_item("__lf_input_json", json_text(&input))
                .map_err(|error| self.error(error.to_string()))?;
            locals
                .set_item("__lf_data_json", json_text(&Value::Object(data)))
                .map_err(|error| self.error(error.to_string()))?;
            locals
                .set_item("__lf_node_id", &ctx.node.id)
                .map_err(|error| self.error(error.to_string()))?;
            locals
                .set_item("__lf_tag_json", json_text(&json!(ctx.node.tag)))
                .map_err(|error| self.error(error.to_string()))?;
            locals
                .set_item(
                    "__lf_loop_index_json",
                    json_text(&json!(ctx.frame.loop_index())),
                )
                .map_err(|error| self.error(error.to_string()))?;
            locals
                .set_item(
                    "__lf_loop_object_json",
                    json_text(&ctx.frame.loop_object().cloned().unwrap_or(Value::Null)),
                )
                .map_err(|error| self.error(error.to_string()))?;

            python
                .run(&code, None, Some(&locals))
                .map_err(|error| self.error(format!("eval error: {error}")))?;
            let envelope = locals
                .get_item("__lf_envelope")
                .map_err(|error| self.error(error.to_string()))?
                .ok_or_else(|| self.error("python envelope missing"))?
                .extract::<String>()
                .map_err(|error| self.error(error.to_string()))?;
            let envelope: Value = serde_json::from_str(&envelope)
                .map_err(|error| self.error(format!("result json error: {error}")))?;
            if let Some(data) = envelope.get("data").and_then(Value::as_object) {
                for (key, value) in data {
                    ctx.inner.data.insert(key.clone(), value.clone());
                }
            }
            Ok(envelope.get("result").cloned().unwrap_or(Value::Null))
        })
    }

    fn error(&self, message: impl Into<String>) -> LiteflowError {
        LiteflowError::Script {
            node: self.node_id.clone(),
            msg: message.into(),
        }
    }
}

#[async_trait]
impl NodeComponent for PythonScriptExecutor {
    async fn process(&self, ctx: &CmpContext) -> LFResult<Value> {
        self.kind.check_return(&self.node_id, self.execute(ctx)?)
    }

    fn name(&self) -> &str {
        &self.node_id
    }
}

/// 对齐 Java PythonScriptExecutor#convertScript：去掉公共首行缩进，并将
/// return 表达式改写为 result 赋值。
fn convert_script(script: &str) -> String {
    let non_blank = script
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    let indent = non_blank
        .first()
        .map(|line| line.len() - line.trim_start().len())
        .unwrap_or(0);
    non_blank
        .into_iter()
        .map(|line| {
            let line = line.get(indent..).unwrap_or(line);
            let trimmed = line.trim_start();
            if let Some(expression) = trimmed.strip_prefix("return ") {
                format!(
                    "{}result = {expression}",
                    &line[..line.len() - trimmed.len()]
                )
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn json_text(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
}
