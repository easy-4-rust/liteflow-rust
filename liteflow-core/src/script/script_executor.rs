//! 对应 ScriptExecutor（JSR223ScriptExecutor 的 Rust 化）：rhai 引擎。
//!
//! 注入到脚本作用域的变量（对齐 Java ScriptExecuteWrap 的绑定）：
//! - `input`     — requestData
//! - `data`      — 链路共享数据（Map；脚本结束后变更合并回上下文）
//! - `node_id`   — 节点 id
//! - `tag`       — 节点 tag
//! - `loop_index`— 循环下标（非循环内为 ()）
//! 脚本最后一个表达式的值为节点返回值。

use super::json_convert::{dynamic_to_json, json_to_dynamic};
use crate::exception::{LFResult, LiteflowError};
use crate::slot::CmpContext;
use rhai::{AST, Dynamic, Engine, Scope};
use serde_json::Value;

pub struct RhaiScriptExecutor {
    engine: Engine,
}

impl Default for RhaiScriptExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl RhaiScriptExecutor {
    pub fn new() -> Self {
        Self {
            engine: Engine::new(),
        }
    }

    /// 对应脚本编译（Java 版缓存编译产物，isCompiled）
    pub fn compile(&self, node_id: &str, script: &str) -> LFResult<AST> {
        self.engine
            .compile(script)
            .map_err(|e| LiteflowError::Script {
                node: node_id.to_string(),
                msg: format!("compile error: {e}"),
            })
    }

    /// 对应 ScriptValidator.validate(script)（只关心是否通过）
    pub fn validate(&self, script: &str) -> bool {
        self.engine.compile(script).is_ok()
    }

    /// 对应 ScriptValidator.validateWithEx(script)（2.16：返回带错误信息的校验结果）
    pub fn validate_ex(&self, script: &str) -> LFResult<()> {
        self.engine.compile(script).map(|_| ()).map_err(|e| LiteflowError::Script {
            node: String::new(),
            msg: format!("script validate failure: {e}"),
        })
    }

    /// 对应 ScriptExecutor.execute
    pub fn execute(&self, node_id: &str, ast: &AST, ctx: &CmpContext) -> LFResult<Value> {
        let mut scope = Scope::new();

        // input：requestData
        let input = ctx
            .inner
            .input
            .lock()
            .map(|v| v.clone())
            .unwrap_or(Value::Null);
        scope.push("input", json_to_dynamic(&input));

        // data：链路共享数据快照
        let mut data_map = rhai::Map::new();
        for r in ctx.inner.data.iter() {
            data_map.insert(r.key().clone().into(), json_to_dynamic(r.value()));
        }
        scope.push("data", Dynamic::from(data_map));

        scope.push("node_id", Dynamic::from(ctx.node.id.clone()));
        scope.push(
            "tag",
            match &ctx.node.tag {
                Some(t) => Dynamic::from(t.clone()),
                None => Dynamic::UNIT,
            },
        );
        scope.push(
            "loop_index",
            match ctx.frame.loop_index() {
                Some(i) => Dynamic::from(i as i64),
                None => Dynamic::UNIT,
            },
        );
        // loop_object
        scope.push(
            "loop_object",
            ctx.frame
                .loop_object()
                .map(json_to_dynamic)
                .unwrap_or(Dynamic::UNIT),
        );

        let result = self
            .engine
            .eval_ast_with_scope::<Dynamic>(&mut scope, ast)
            .map_err(|e| LiteflowError::Script {
                node: node_id.to_string(),
                msg: format!("eval error: {e}"),
            })?;

        // data 变更合并回上下文（对齐 Java 脚本直接操作上下文 bean 的语义）
        if let Some(m) = scope.get_value::<rhai::Map>("data") {
            for (k, v) in m {
                ctx.inner.data.insert(k.to_string(), dynamic_to_json(&v));
            }
        }

        Ok(dynamic_to_json(&result))
    }
}
