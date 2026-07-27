//! Rhai 脚本执行器。
//!
//! 注入到脚本作用域的变量对齐 Java `ScriptExecuteWrap`：
//! `input`、`data`、`node_id`、`tag`、`loop_index` 与 `loop_object`。

use std::collections::HashMap;
use std::sync::RwLock;

use super::ScriptExecutor;
use super::json_convert::{dynamic_to_json, json_to_dynamic};
use crate::common::entity::ValidationResp;
use crate::enums::ScriptTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::slot::CmpContext;
use chrono::Local;
use rhai::{AST, Dynamic, Engine, EvalAltResult, Position, Scope};
use serde_json::Value;

/// 基于 Rhai 的脚本执行器，按节点 id 缓存强类型 AST。
///
/// 这是 Rust 生态对 Java `ScriptExecutor`/QLExpress 执行器生态位的具体实现。
pub struct RhaiScriptExecutor {
    engine: Engine,
    compiled_script_map: RwLock<HashMap<String, AST>>,
}

impl Default for RhaiScriptExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl RhaiScriptExecutor {
    /// 创建 Rhai 引擎并注册受控脚本 Bean 调用桥。
    #[must_use]
    pub fn new() -> Self {
        let mut engine = Engine::new();
        // Rust 无运行期反射，因此通过统一桥接函数访问受控脚本 Bean。
        // 方法筛选与别名已经在 ScriptBeanProxy 构建阶段固化。
        engine.register_fn(
            "script_call",
            |bean_name: &str,
             method_name: &str,
             arguments: rhai::Array|
             -> Result<Dynamic, Box<EvalAltResult>> {
                let arguments = arguments.iter().map(dynamic_to_json).collect::<Vec<_>>();
                crate::script::ScriptBeanManager::invoke(bean_name, method_name, &arguments)
                    .map(|value| json_to_dynamic(&value))
                    .map_err(|error| {
                        EvalAltResult::ErrorRuntime(format!("{error}").into(), Position::NONE)
                            .into()
                    })
            },
        );
        // Aviator 基线脚本中的 DateUtil.formatDateTime(new Date()) 在适配层被映射到
        // 此运行时函数，确保时间在每次执行时生成，而不是在脚本编译时冻结。
        engine.register_fn("aviator_now", || {
            Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
        });
        Self {
            engine,
            compiled_script_map: RwLock::new(HashMap::new()),
        }
    }

    /// 编译指定节点的脚本并返回强类型 AST。
    ///
    /// `node_id` 用于定位编译错误，`script` 为源代码。对应 Java:
    /// `ScriptExecutor#compile`。
    pub fn compile(&self, node_id: &str, script: &str) -> LFResult<AST> {
        self.engine
            .compile(script)
            .map_err(|error| LiteflowError::Script {
                node: node_id.to_string(),
                msg: format!("compile error: {error}"),
            })
    }

    /// 校验脚本并以 Rust `Result` 保留错误。
    ///
    /// `script` 为待校验文本；成功时返回空值。
    pub fn validate_ex(&self, script: &str) -> LFResult<()> {
        self.engine
            .compile(script)
            .map(|_| ())
            .map_err(|error| LiteflowError::Script {
                node: String::new(),
                msg: format!("script validate failure: {error}"),
            })
    }

    /// 校验脚本是否能够被 Rhai 编译。
    ///
    /// `script` 为待校验文本，只返回成功与否。对应 Java:
    /// `ScriptExecutor#validate`。
    #[must_use]
    pub fn validate(&self, script: &str) -> bool {
        self.validate_ex(script).is_ok()
    }

    /// 校验脚本并返回包含编译失败原因的响应。
    ///
    /// `script` 为待校验文本。对应 Java: `ScriptExecutor#validate`。
    #[must_use]
    pub fn validate_with_ex(&self, script: &str) -> ValidationResp {
        match self.validate_ex(script) {
            Ok(()) => ValidationResp::success(),
            Err(error) => ValidationResp::fail(error),
        }
    }

    fn cache_read_error() -> LiteflowError {
        LiteflowError::Script {
            node: String::new(),
            msg: "rhai script cache read lock poisoned".to_string(),
        }
    }

    fn cache_write_error() -> LiteflowError {
        LiteflowError::Script {
            node: String::new(),
            msg: "rhai script cache write lock poisoned".to_string(),
        }
    }

    fn evaluate(&self, node_id: &str, ast: &AST, ctx: &CmpContext) -> LFResult<Value> {
        let mut scope = Scope::new();

        // requestData 映射为 input；锁异常时沿用当前框架的空输入退化语义。
        let input = ctx
            .inner
            .input
            .lock()
            .map(|value| value.clone())
            .unwrap_or(Value::Null);
        scope.push("input", json_to_dynamic(&input));

        // 把链路共享数据快照注入脚本，执行完毕后再合并回并发上下文。
        let mut data_map = rhai::Map::new();
        for entry in ctx.inner.data.iter() {
            data_map.insert(entry.key().clone().into(), json_to_dynamic(entry.value()));
        }
        scope.push("data", Dynamic::from(data_map));
        scope.push("node_id", Dynamic::from(ctx.node.id.clone()));
        scope.push(
            "tag",
            ctx.node
                .tag
                .clone()
                .map(Dynamic::from)
                .unwrap_or(Dynamic::UNIT),
        );
        scope.push(
            "loop_index",
            ctx.frame
                .loop_index()
                .map(|index| Dynamic::from(index as i64))
                .unwrap_or(Dynamic::UNIT),
        );
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
            .map_err(|error| LiteflowError::Script {
                node: node_id.to_string(),
                msg: format!("eval error: {error}"),
            })?;

        // 脚本对 data 的修改合并回上下文，对齐 Java 脚本直接操作上下文 Bean。
        if let Some(data) = scope.get_value::<rhai::Map>("data") {
            for (key, value) in data {
                ctx.inner
                    .data
                    .insert(key.to_string(), dynamic_to_json(&value));
            }
        }

        Ok(dynamic_to_json(&result))
    }
}

impl ScriptExecutor for RhaiScriptExecutor {
    fn load(&self, node_id: &str, script: &str) -> LFResult<()> {
        let ast = self.compile(node_id, script)?;
        self.compiled_script_map
            .write()
            .map_err(|_| Self::cache_write_error())?
            .insert(node_id.to_string(), ast);
        Ok(())
    }

    fn unload(&self, node_id: &str) -> LFResult<()> {
        self.compiled_script_map
            .write()
            .map_err(|_| Self::cache_write_error())?
            .remove(node_id);
        Ok(())
    }

    fn node_ids(&self) -> LFResult<Vec<String>> {
        let mut node_ids = self
            .compiled_script_map
            .read()
            .map_err(|_| Self::cache_read_error())?
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        node_ids.sort();
        Ok(node_ids)
    }

    fn execute_script(&self, node_id: &str, ctx: &CmpContext) -> LFResult<Value> {
        let ast = self
            .compiled_script_map
            .read()
            .map_err(|_| Self::cache_read_error())?
            .get(node_id)
            .cloned()
            .ok_or_else(|| LiteflowError::Script {
                node: node_id.to_string(),
                msg: format!("script for node[{node_id}] is not loaded"),
            })?;
        self.evaluate(node_id, &ast, ctx)
    }

    fn clean_cache(&self) -> LFResult<()> {
        self.compiled_script_map
            .write()
            .map_err(|_| Self::cache_write_error())?
            .clear();
        Ok(())
    }

    fn script_type(&self) -> ScriptTypeEnum {
        ScriptTypeEnum::Rhai
    }

    fn validate_with_ex(&self, script: &str) -> ValidationResp {
        RhaiScriptExecutor::validate_with_ex(self, script)
    }
}
