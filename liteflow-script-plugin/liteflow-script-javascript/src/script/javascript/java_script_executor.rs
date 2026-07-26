//! 对应 Java: `com.yomahub.liteflow.script.javascript.JavaScriptExecutor`。
//!
//! 使用纯 Rust Boa 引擎，不开放宿主文件系统或网络能力。

use std::sync::Arc;

use async_trait::async_trait;
use boa_engine::{Context, Source};
use liteflow_core::core::NodeComponent;
use liteflow_core::script::{ScriptExecutorFactory, ScriptKind};
use liteflow_core::{CmpContext, LFResult, LiteflowError};
use serde_json::{Value, json};

/// JavaScript 脚本执行组件。
pub struct JavaScriptExecutor {
    node_id: String,
    kind: ScriptKind,
    script: String,
}

impl JavaScriptExecutor {
    /// 注册 Java 插件使用的 `js` displayName，同时接受 `javascript` 别名。
    pub fn register() -> LFResult<()> {
        ScriptExecutorFactory::register("js", Self::build)?;
        ScriptExecutorFactory::register("javascript", Self::build)
    }

    /// 为兼容运行时注册额外语言别名。
    ///
    /// 对应 Java `ScriptExecutorFactory` 按脚本引擎名称缓存执行器。
    pub fn register_language(language: impl Into<String>) -> LFResult<()> {
        ScriptExecutorFactory::register(language, Self::build)
    }

    fn build(node_id: &str, kind: ScriptKind, script: &str) -> LFResult<Arc<dyn NodeComponent>> {
        // new Function 只编译函数体而不执行，等价 Java executor.load。
        let source = format!(
            "new Function({})",
            serde_json::to_string(script).unwrap_or_else(|_| "\"\"".to_string())
        );
        Context::default()
            .eval(Source::from_bytes(source.as_bytes()))
            .map_err(|error| LiteflowError::Script {
                node: node_id.to_string(),
                msg: format!("compile error: {error}"),
            })?;
        Ok(Arc::new(Self {
            node_id: node_id.to_string(),
            kind,
            script: script.to_string(),
        }))
    }

    /// 执行脚本并把 data 对象写回上下文。
    ///
    /// 对应 Java `JSR223ScriptExecutor#executeScript` 的 bindings 注入。
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
            const input = {};
            let data = {};
            const node_id = {};
            const tag = {};
            const loop_index = {};
            const loop_object = {};
            const __liteflow_result = (function() {{ {} }})();
            JSON.stringify({{
                result: __liteflow_result === undefined ? null : __liteflow_result,
                data: data
            }});
            "#,
            json_text(&input),
            json_text(&Value::Object(data)),
            json_text(&json!(ctx.node.id)),
            json_text(&json!(ctx.node.tag)),
            json_text(&json!(ctx.frame.loop_index())),
            json_text(&ctx.frame.loop_object().cloned().unwrap_or(Value::Null)),
            self.script,
        );

        let mut context = Context::default();
        let result = context
            .eval(Source::from_bytes(runtime.as_bytes()))
            .map_err(|error| self.error(format!("eval error: {error}")))?;
        let result = result
            .to_string(&mut context)
            .map_err(|error| self.error(format!("result conversion error: {error}")))?
            .to_std_string_escaped();
        let envelope: Value = serde_json::from_str(&result)
            .map_err(|error| self.error(format!("result json error: {error}")))?;
        if let Some(data) = envelope.get("data").and_then(Value::as_object) {
            for (key, value) in data {
                ctx.inner.data.insert(key.clone(), value.clone());
            }
        }
        Ok(envelope.get("result").cloned().unwrap_or(Value::Null))
    }

    fn error(&self, message: impl Into<String>) -> LiteflowError {
        LiteflowError::Script {
            node: self.node_id.clone(),
            msg: message.into(),
        }
    }
}

#[async_trait]
impl NodeComponent for JavaScriptExecutor {
    async fn process(&self, ctx: &CmpContext) -> LFResult<Value> {
        self.kind.check_return(&self.node_id, self.execute(ctx)?)
    }

    fn name(&self) -> &str {
        &self.node_id
    }
}

fn json_text(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
}
