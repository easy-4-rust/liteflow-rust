//! Rhai 脚本执行器。
//!
//! 注入到脚本作用域的变量对齐 Java `ScriptExecuteWrap`：
//! `input`、`data`、`node_id`、`tag`、`loop_index`、`loop_object` 与 `_meta`。

use std::collections::HashMap;
use std::sync::RwLock;

use super::ScriptExecutor;
use super::json_convert::{dynamic_to_json, json_to_dynamic};
use crate::common::entity::ValidationResp;
use crate::enums::ScriptTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::script::proxy::ScriptBeanProxy;
use crate::slot::CmpContext;
use chrono::Local;
use rhai::{AST, Dynamic, Engine, EvalAltResult, Position, Scope};
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone, Default)]
struct ScriptBeanBindings {
    beans: HashMap<String, Arc<ScriptBeanProxy>>,
}

/// 基于 Rhai 的脚本执行器，按节点 id 缓存强类型 AST。
///
/// 这是 LiteFlow 内置的 Rhai 脚本实现；Java QLExpress 插件已由独立
/// `liteflow-script-qlexpress` crate 接入真实 `qlexpress` Rust 运行时。
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
        engine.register_type_with_name::<ScriptBeanBindings>("ScriptBeanBindings");
        engine.register_fn(
            "script_context_call",
            |bindings: &mut ScriptBeanBindings,
             bean_name: &str,
             method_name: &str,
             arguments: rhai::Array|
             -> Result<Dynamic, Box<EvalAltResult>> {
                let arguments = arguments.iter().map(dynamic_to_json).collect::<Vec<_>>();
                let result = bindings
                    .beans
                    .get(bean_name)
                    .map_or_else(
                        || {
                            crate::script::ScriptBeanManager::invoke(
                                bean_name,
                                method_name,
                                &arguments,
                            )
                        },
                        |bean| bean.invoke(method_name, &arguments),
                    )
                    .map(|value| json_to_dynamic(&value));
                result.map_err(|error| {
                    EvalAltResult::ErrorRuntime(format!("{error}").into(), Position::NONE).into()
                })
            },
        );
        // Aviator 基线脚本中的 DateUtil.formatDateTime(new Date()) 在适配层被映射到
        // 此运行时函数，确保时间在每次执行时生成，而不是在脚本编译时冻结。
        engine.register_fn("aviator_now", || {
            Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
        });
        engine.register_fn(
            "kotlin_to_int",
            |value: Dynamic| -> Result<i64, Box<EvalAltResult>> {
                let value = dynamic_to_json(&value);
                match value {
                    Value::Number(number) => number.as_i64().ok_or_else(|| {
                        EvalAltResult::ErrorRuntime(
                            "Kotlin Int conversion overflow".into(),
                            Position::NONE,
                        )
                        .into()
                    }),
                    Value::String(value) => value.parse::<i64>().map_err(|error| {
                        EvalAltResult::ErrorRuntime(
                            format!("Kotlin String.toInt failed: {error}").into(),
                            Position::NONE,
                        )
                        .into()
                    }),
                    other => Err(EvalAltResult::ErrorRuntime(
                        format!("Kotlin toInt does not accept {other}").into(),
                        Position::NONE,
                    )
                    .into()),
                }
            },
        );
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

        // 所有 serde 可表达的上下文 Bean 和 Java `_meta` 字段统一由
        // ScriptExecutor#bindParam 构建，Rhai 不再维护一份容易漂移的私有映射。
        for (name, value) in <Self as ScriptExecutor>::bind_param(self, ctx) {
            scope.push_dynamic(name, json_to_dynamic(&value));
        }

        // Java JSR223 会把本次执行的 context bean 与进程级 ScriptBean 一起绑定。
        // Rust 将 Slot 中的 ScriptBeanProxy 做成调用快照，优先级高于全局注册表，
        // 避免并发请求之间通过临时全局状态相互污染。
        let script_beans = ctx
            .inner
            .beans
            .iter()
            .filter_map(|entry| {
                Arc::clone(entry.value())
                    .downcast::<ScriptBeanProxy>()
                    .ok()
                    .map(|proxy| (entry.key().clone(), proxy))
            })
            .collect();
        scope.push(
            "_script_beans",
            ScriptBeanBindings {
                beans: script_beans,
            },
        );

        // 把链路共享数据快照注入脚本，执行完毕后再合并回并发上下文。
        let mut data_map = rhai::Map::new();
        for entry in ctx.inner.data.iter() {
            data_map.insert(entry.key().clone().into(), json_to_dynamic(entry.value()));
        }
        scope.push("data", Dynamic::from(data_map));

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
    /// 使用 Rhai 编译器生成 AST，但不修改节点缓存。
    ///
    /// 参数 `script` 是待编译源代码。对应 Java: `ScriptExecutor#compile`。
    fn compile(&self, script: &str) -> LFResult<()> {
        RhaiScriptExecutor::compile(self, "", script).map(|_| ())
    }

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
