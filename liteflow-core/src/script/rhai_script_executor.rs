//! Rhai 脚本执行器。
//!
//! 注入到脚本作用域的变量对齐 Java `ScriptExecuteWrap`：
//! `input`、`data`、`node_id`、`tag`、`loop_index`、`loop_object` 与 `_meta`。

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

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

#[derive(Clone, Default)]
struct ScriptBeanBindings {
    beans: HashMap<String, Arc<ScriptBeanProxy>>,
    context_beans: HashMap<String, Arc<RwLock<Value>>>,
}

/// Java `bindings["defaultContext"]` 的请求级数据桥。
///
/// 该私有适配器只服务于脚本运行时，不对应独立 Java 对象。它持有当前
/// `CmpContext` 的克隆句柄，使 Kotlin 函数即使拥有独立局部作用域，也能继续
/// 读取和修改同一个 DefaultContext。
#[derive(Clone, Default)]
struct ScriptDataBindings {
    context: Option<CmpContext>,
    writes: Arc<Mutex<HashMap<String, Value>>>,
}

/// 脚本运行时抛出的 LiteFlow 业务异常载荷。
///
/// 该私有桥接对象让受控 Kotlin/Groovy 等适配器无需 JVM 反射，也能把脚本中的
/// `LiteFlowException(code, message)` 还原为 Rust `LiteFlowException`。
#[derive(Clone)]
struct ScriptLiteFlowException {
    code: Option<String>,
    message: String,
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
                let result = if let Some(bean) = bindings.beans.get(bean_name) {
                    bean.invoke(method_name, &arguments)
                } else if let Some(bean) = bindings.context_beans.get(bean_name) {
                    invoke_serde_context_bean(bean_name, bean, method_name, &arguments)
                } else {
                    crate::script::ScriptBeanManager::invoke(bean_name, method_name, &arguments)
                }
                .map(|value| json_to_dynamic(&value));
                result.map_err(|error| {
                    EvalAltResult::ErrorRuntime(format!("{error}").into(), Position::NONE).into()
                })
            },
        );
        engine.register_type_with_name::<ScriptDataBindings>("ScriptDataBindings");
        engine.register_fn(
            "script_data_get",
            |bindings: &mut ScriptDataBindings, key: &str| -> Dynamic {
                if let Ok(writes) = bindings.writes.lock()
                    && let Some(value) = writes.get(key)
                {
                    return json_to_dynamic(value);
                }
                bindings
                    .context
                    .as_ref()
                    .and_then(|context| context.get_data(key).map(|value| json_to_dynamic(&value)))
                    .unwrap_or(Dynamic::UNIT)
            },
        );
        engine.register_fn(
            "script_data_has",
            |bindings: &mut ScriptDataBindings, key: &str| -> bool {
                if bindings
                    .writes
                    .lock()
                    .is_ok_and(|writes| writes.contains_key(key))
                {
                    return true;
                }
                bindings
                    .context
                    .as_ref()
                    .is_some_and(|context| context.get_data(key).is_some())
            },
        );
        engine.register_fn(
            "script_data_set",
            |bindings: &mut ScriptDataBindings, key: &str, value: Dynamic| {
                if let Ok(mut writes) = bindings.writes.lock() {
                    writes.insert(key.to_string(), dynamic_to_json(&value));
                }
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
        engine.register_type_with_name::<ScriptLiteFlowException>("ScriptLiteFlowException");
        engine.register_fn(
            "liteflow_throw",
            |message: &str| -> Result<Dynamic, Box<EvalAltResult>> {
                Err(EvalAltResult::ErrorRuntime(
                    Dynamic::from(ScriptLiteFlowException {
                        code: None,
                        message: message.to_string(),
                    }),
                    Position::NONE,
                )
                .into())
            },
        );
        engine.register_fn(
            "liteflow_throw",
            |code: &str, message: &str| -> Result<Dynamic, Box<EvalAltResult>> {
                Err(EvalAltResult::ErrorRuntime(
                    Dynamic::from(ScriptLiteFlowException {
                        code: Some(code.to_string()),
                        message: message.to_string(),
                    }),
                    Position::NONE,
                )
                .into())
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
        let mut script_beans = HashMap::new();
        let mut context_beans = HashMap::new();
        for entry in &ctx.inner.beans {
            if let Ok(proxy) = Arc::clone(entry.value()).downcast::<ScriptBeanProxy>() {
                script_beans.insert(entry.key().clone(), proxy);
            } else if let Ok(bean) = Arc::clone(entry.value()).downcast::<RwLock<Value>>() {
                context_beans.insert(entry.key().clone(), bean);
            }
        }
        scope.push(
            "_script_beans",
            ScriptBeanBindings {
                beans: script_beans,
                context_beans,
            },
        );
        let script_data_writes = Arc::new(Mutex::new(HashMap::new()));
        scope.push(
            "_script_data",
            ScriptDataBindings {
                context: Some(ctx.clone()),
                writes: Arc::clone(&script_data_writes),
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
            .map_err(|error| map_script_eval_error(node_id, error.as_ref()))?;

        // 脚本对 data 的修改合并回上下文，对齐 Java 脚本直接操作上下文 Bean。
        if let Some(data) = scope.get_value::<rhai::Map>("data") {
            for (key, value) in data {
                ctx.inner
                    .data
                    .insert(key.to_string(), dynamic_to_json(&value));
            }
        }
        // Kotlin 函数通过请求级桥写入的值在通用 data 快照之后提交，避免旧快照
        // 覆盖函数内的 setData；同一脚本中的后续 getData 会优先读取这份写集。
        if let Ok(writes) = script_data_writes.lock() {
            for (key, value) in writes.iter() {
                ctx.inner.data.insert(key.clone(), value.clone());
            }
        }

        Ok(dynamic_to_json(&result))
    }
}

/// 调用 serde 上下文 Bean 的 JavaBean getter/setter。
///
/// Rust 以 `Arc<RwLock<serde_json::Value>>` 映射 Java 可变上下文对象：读写均作用
/// 于 Slot 中的同一实例，脚本结束后响应仍可取得更新值。上下文 Bean 优先于同名
/// 全局 ScriptBean，与 Java `ScriptExecutor#bindParam` 的 `putIfAbsent` 一致。
fn invoke_serde_context_bean(
    bean_name: &str,
    bean: &Arc<RwLock<Value>>,
    method_name: &str,
    arguments: &[Value],
) -> LFResult<Value> {
    let mut bean = bean.write().map_err(|_| LiteflowError::Script {
        node: bean_name.to_string(),
        msg: "serde context bean write lock poisoned".to_string(),
    })?;
    let object = bean.as_object_mut().ok_or_else(|| LiteflowError::Script {
        node: bean_name.to_string(),
        msg: format!("context bean[{bean_name}] must be a JSON object"),
    })?;

    if let Some(property) = java_bean_property_name(method_name, "set") {
        if arguments.len() != 1 {
            return Err(LiteflowError::Script {
                node: bean_name.to_string(),
                msg: format!(
                    "context bean setter[{method_name}] requires 1 argument, got {}",
                    arguments.len()
                ),
            });
        }
        object.insert(property, arguments[0].clone());
        return Ok(Value::Null);
    }
    if let Some(property) = java_bean_property_name(method_name, "get")
        .or_else(|| java_bean_property_name(method_name, "is"))
    {
        if !arguments.is_empty() {
            return Err(LiteflowError::Script {
                node: bean_name.to_string(),
                msg: format!(
                    "context bean getter[{method_name}] requires 0 arguments, got {}",
                    arguments.len()
                ),
            });
        }
        return Ok(object.get(&property).cloned().unwrap_or(Value::Null));
    }
    Err(LiteflowError::Script {
        node: bean_name.to_string(),
        msg: format!(
            "context bean method[{method_name}] is outside the serde JavaBean getter/setter boundary"
        ),
    })
}

/// 按 JavaBeans Introspector.decapitalize 规则把方法后缀转换为属性名。
fn java_bean_property_name(method_name: &str, prefix: &str) -> Option<String> {
    let suffix = method_name.strip_prefix(prefix)?;
    let mut characters = suffix.chars();
    let first = characters.next()?;
    if !first.is_ascii_uppercase() {
        return None;
    }
    if characters
        .clone()
        .next()
        .is_some_and(|second| second.is_ascii_uppercase())
    {
        return Some(suffix.to_string());
    }
    Some(format!(
        "{}{}",
        first.to_ascii_lowercase(),
        characters.collect::<String>()
    ))
}

/// 把 Rhai 调用栈中的受控业务异常还原为 LiteFlowException。
fn map_script_eval_error(node_id: &str, error: &EvalAltResult) -> LiteflowError {
    if let Some(exception) = find_script_liteflow_exception(error) {
        let exception = match exception.code {
            Some(code) => crate::exception::LiteFlowException::with_code(code, exception.message),
            None => crate::exception::LiteFlowException::new(exception.message),
        };
        return LiteflowError::LiteFlow(exception);
    }
    LiteflowError::Script {
        node: node_id.to_string(),
        msg: format!("eval error: {error}"),
    }
}

/// 递归穿透 Rhai 函数/模块包装，提取真正的脚本业务异常。
fn find_script_liteflow_exception(error: &EvalAltResult) -> Option<ScriptLiteFlowException> {
    match error {
        EvalAltResult::ErrorRuntime(value, _) => {
            value.clone().try_cast::<ScriptLiteFlowException>()
        }
        EvalAltResult::ErrorInFunctionCall(_, _, source, _)
        | EvalAltResult::ErrorInModule(_, source, _) => find_script_liteflow_exception(source),
        _ => None,
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
