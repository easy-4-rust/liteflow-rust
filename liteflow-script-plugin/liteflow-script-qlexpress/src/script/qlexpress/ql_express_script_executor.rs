//! QLExpress 脚本执行器。

// qlexpress 统一使用携带完整诊断信息的 QLException；宿主扩展 trait 的错误类型不可改为 Box。
#![allow(clippy::result_large_err)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, OnceLock, RwLock};

use liteflow_core::common::entity::ValidationResp;
use liteflow_core::enums::ScriptTypeEnum;
use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::script::proxy::ScriptBeanProxy;
use liteflow_core::script::{
    ScriptBeanManager, ScriptExecutor, ScriptExecutorComponent, ScriptExecutorFactory, ScriptKind,
};
use liteflow_core::slot::CmpContext;
use qlexpress::api::parsecache::SerializableParseCache;
use qlexpress::runtime::data::index_map::IndexMap;
use qlexpress::{
    DataValue, Express4Runner, InitOptions, NativeObject, QLException, QLExceptionKind, QLOptions,
    QLSecurityStrategy,
};
use serde_json::{Map, Number, Value, json};

const GET_DATA_FUNCTION: &str = "__liteflow_get_data";
const HAS_DATA_FUNCTION: &str = "__liteflow_has_data";
const SET_DATA_FUNCTION: &str = "__liteflow_set_data";
const META_FUNCTION: &str = "__liteflow_meta";
const PRINTLN_FUNCTION: &str = "__liteflow_println";
const SCRIPT_BEAN_TYPE: &str = "com.yomahub.liteflow.script.proxy.ScriptBeanProxy";

/// 阿里 QLExpress 脚本语言的 Rust 执行器。
///
/// 该执行器直接使用已发布的 `qlexpress` crate 执行词法分析、语法分析、编译与 QVM
/// 指令，不再维护 LiteFlow 私有的表达式解释器。全局执行器仅缓存可跨线程传递的
/// `SerializableParseCache`；每次执行创建局部 `Express4Runner`，从而遵守真实
/// QLExpress Rust Runner 的 `Rc/RefCell` 单线程边界。对应 Java:
/// `com.yomahub.liteflow.script.qlexpress.QLExpressScriptExecutor`。
pub struct QlExpressScriptExecutor {
    compiled_script_map: RwLock<HashMap<String, SerializableParseCache>>,
}

impl Default for QlExpressScriptExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl QlExpressScriptExecutor {
    /// 创建空的 QLExpress 执行器。
    ///
    /// 返回值是尚未加载脚本的线程安全执行器。对应 Java:
    /// `QLExpressScriptExecutor#init`。
    #[must_use]
    pub fn new() -> Self {
        Self {
            compiled_script_map: RwLock::new(HashMap::new()),
        }
    }

    /// 注册 `qlexpress` 语言组件构建器。
    ///
    /// 返回注册结果；重复注册由 `ScriptExecutorFactory` 按统一规则处理。
    pub fn register() -> LFResult<()> {
        ScriptExecutorFactory::register("qlexpress", Self::build)
    }

    /// 用真实 QLExpress 编译器生成可序列化指令缓存。
    ///
    /// `script` 为 LiteFlow 节点脚本；返回值可安全存放在全局执行器中。对应 Java:
    /// `Express4Runner#parseToSerializableCache`。
    fn compile(&self, script: &str) -> Result<SerializableParseCache, QLException> {
        let source = normalize_liteflow_calls(script);
        let runner = create_runner();
        register_context_functions(&runner, None);
        register_global_script_bean_functions(&runner);
        runner.export_parse_cache(&source)
    }

    fn build(
        node_id: &str,
        kind: ScriptKind,
        script: &str,
    ) -> LFResult<Arc<dyn liteflow_core::NodeComponent>> {
        let executor = shared_executor();
        executor.load(node_id, script)?;
        let executor: Arc<dyn ScriptExecutor> = executor;
        Ok(Arc::new(ScriptExecutorComponent::new(
            node_id, kind, executor,
        )))
    }
}

impl ScriptExecutor for QlExpressScriptExecutor {
    /// 使用发布版 QLExpress Rust 编译器生成真实指令缓存，但不写入节点缓存。
    ///
    /// 参数 `script` 是待编译的 QLExpress 源代码。对应 Java:
    /// `ScriptExecutor#compile`。
    fn compile(&self, script: &str) -> LFResult<()> {
        QlExpressScriptExecutor::compile(self, script)
            .map(|_| ())
            .map_err(|error| script_error("", error))
    }

    /// 编译并缓存节点脚本。
    ///
    /// `node_id` 是缓存键，`script` 是原始 QLExpress 文本；语法错误直接返回
    /// LiteFlow 脚本异常。对应 Java: `QLExpressScriptExecutor#load`。
    fn load(&self, node_id: &str, script: &str) -> LFResult<()> {
        let cache = self
            .compile(script)
            .map_err(|error| script_error(node_id, error))?;
        self.compiled_script_map
            .write()
            .map_err(|_| cache_error("write"))?
            .insert(node_id.to_string(), cache);
        Ok(())
    }

    /// 卸载指定节点的真实编译缓存。
    ///
    /// `node_id` 为待卸载节点；节点不存在时保持幂等。对应 Java:
    /// `QLExpressScriptExecutor#unLoad`。
    fn unload(&self, node_id: &str) -> LFResult<()> {
        self.compiled_script_map
            .write()
            .map_err(|_| cache_error("write"))?
            .remove(node_id);
        Ok(())
    }

    /// 返回已加载节点 ID。
    ///
    /// 返回值按字典序排序，避免并发 Map 迭代顺序影响调用方。对应 Java:
    /// `QLExpressScriptExecutor#getNodeIds`。
    fn node_ids(&self) -> LFResult<Vec<String>> {
        let mut node_ids = self
            .compiled_script_map
            .read()
            .map_err(|_| cache_error("read"))?
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        node_ids.sort();
        Ok(node_ids)
    }

    /// 使用 LiteFlow 上下文执行真实 QLExpress 编译产物。
    ///
    /// `node_id` 用于定位缓存，`context` 提供请求数据、节点元数据、共享数据和
    /// ScriptBean；返回脚本最终值。对应 Java:
    /// `QLExpressScriptExecutor#executeScript`。
    fn execute_script(&self, node_id: &str, context: &CmpContext) -> LFResult<Value> {
        let cache = self
            .compiled_script_map
            .read()
            .map_err(|_| cache_error("read"))?
            .get(node_id)
            .cloned()
            .ok_or_else(|| LiteflowError::Script {
                node: node_id.to_string(),
                msg: format!("script for node[{node_id}] is not loaded"),
            })?;

        // Runner 含 Rc/RefCell，只在当前执行线程内创建和销毁；缓存本身是纯 serde 数据。
        let runner = create_runner();
        register_context_functions(&runner, Some(context.clone()));
        register_global_script_bean_functions(&runner);
        let ql_context = build_ql_context(&cache, context);
        let result = runner
            .execute_with_cache(
                &cache,
                Rc::new(qlexpress::MapExpressContext::new(Rc::new(RefCell::new(
                    hash_map_to_index_map(ql_context),
                )))),
                &QLOptions::default(),
            )
            .map_err(|error| script_error(node_id, error))?;
        data_value_to_json(result.result()).map_err(|message| LiteflowError::Script {
            node: node_id.to_string(),
            msg: message,
        })
    }

    /// 清理全部真实编译缓存。
    ///
    /// 返回锁访问结果。对应 Java: `QLExpressScriptExecutor#cleanCache`。
    fn clean_cache(&self) -> LFResult<()> {
        self.compiled_script_map
            .write()
            .map_err(|_| cache_error("write"))?
            .clear();
        Ok(())
    }

    fn script_type(&self) -> ScriptTypeEnum {
        ScriptTypeEnum::QlExpress
    }

    /// 使用真实 QLExpress 编译器校验源代码并保留完整诊断。
    ///
    /// `script` 是待校验文本；成功返回通过响应，失败返回带 QLExpress 错误码和
    /// 位置信息的响应。对应 Java: `ScriptExecutor#validate`。
    fn validate_with_ex(&self, script: &str) -> ValidationResp {
        match self.compile(script) {
            Ok(_) => ValidationResp::success(),
            Err(error) => ValidationResp::fail(script_error("", error)),
        }
    }
}

/// ScriptBean 的动态原生对象适配器。
///
/// 该私有适配器把 LiteFlow 的 include/exclude 方法规则带入 QLExpress
/// `NativeObject` 分派；它不是 Java 对象迁移文件中的独立公开对象。
struct ScriptBeanNativeObject {
    proxy: Arc<ScriptBeanProxy>,
}

impl NativeObject for ScriptBeanNativeObject {
    fn get_field(&self, _name: &str) -> Option<DataValue> {
        None
    }

    fn call_method(&mut self, name: &str, args: &[DataValue]) -> Result<DataValue, QLException> {
        let arguments = args
            .iter()
            .map(data_value_to_json)
            .collect::<Result<Vec<_>, _>>()
            .map_err(ql_bridge_error)?;
        self.proxy
            .invoke(name, &arguments)
            .map_err(|error| ql_bridge_error(error.to_string()))
            .and_then(|value| json_to_data_value(&value).map_err(ql_bridge_error))
    }

    fn native_type_name(&self) -> &str {
        SCRIPT_BEAN_TYPE
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn create_runner() -> Express4Runner {
    // 仅显式暴露的 ScriptBean 原生对象会进入上下文，代理自身继续执行方法白名单校验。
    let options = InitOptions::builder()
        .security_strategy(QLSecurityStrategy::open())
        .build();
    Express4Runner::with_init_options(options)
}

fn register_context_functions(runner: &Express4Runner, context: Option<CmpContext>) {
    let get_context = context.clone();
    runner.add_function_unary(GET_DATA_FUNCTION, move |key| {
        let Some(context) = &get_context else {
            return DataValue::Null;
        };
        key.as_str()
            .and_then(|key| context.get_data(key))
            .and_then(|value| json_to_data_value(&value).ok())
            .unwrap_or(DataValue::Null)
    });

    let has_context = context.clone();
    runner.add_function_unary(HAS_DATA_FUNCTION, move |key| {
        DataValue::Bool(
            has_context
                .as_ref()
                .zip(key.as_str())
                .is_some_and(|(context, key)| context.get_data(key).is_some()),
        )
    });

    let set_context = context.clone();
    runner.add_function_bi(SET_DATA_FUNCTION, move |key, value| {
        if let (Some(context), Some(key)) = (&set_context, key.as_str())
            && let Ok(json_value) = data_value_to_json(&value)
        {
            context.set_data(key, json_value);
        }
        value
    });

    let meta_context = context;
    runner.add_function_unary(META_FUNCTION, move |key| {
        meta_context
            .as_ref()
            .zip(key.as_str())
            .map_or(DataValue::Null, |(context, key)| {
                json_to_data_value(&meta_value(context, key)).unwrap_or(DataValue::Null)
            })
    });

    runner.add_function_unary(PRINTLN_FUNCTION, |value| value);
}

fn register_global_script_bean_functions(runner: &Express4Runner) {
    for (bean_name, proxy) in ScriptBeanManager::get_script_bean_map() {
        for method_name in proxy.method_names() {
            let function_name = script_bean_function_name(&bean_name, &method_name);
            let proxy = Arc::clone(&proxy);
            let method_name = method_name.clone();
            runner.add_varargs_function(function_name, move |arguments: &[DataValue]| {
                let arguments = arguments
                    .iter()
                    .map(data_value_to_json)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(ql_bridge_error)?;
                proxy
                    .invoke(&method_name, &arguments)
                    .map_err(|error| ql_bridge_error(error.to_string()))
                    .and_then(|value| json_to_data_value(&value).map_err(ql_bridge_error))
            });
        }
    }
}

fn build_ql_context(
    cache: &SerializableParseCache,
    context: &CmpContext,
) -> HashMap<String, DataValue> {
    let mut values = HashMap::new();
    let request_data = context.request_data::<Value>().unwrap_or(Value::Null);
    let request_value = json_to_data_value(&request_data).unwrap_or(DataValue::Null);
    values.insert("requestData".to_string(), request_value.clone());
    values.insert("input".to_string(), request_value);
    values.insert(
        "cmp_data".to_string(),
        context
            .cmp_data()
            .and_then(|value| serde_json::from_str::<Value>(value).ok())
            .and_then(|value| json_to_data_value(&value).ok())
            .unwrap_or(DataValue::Null),
    );
    values.insert(
        "loop_object".to_string(),
        context
            .loop_object::<Value>()
            .and_then(|value| json_to_data_value(&value).ok())
            .unwrap_or(DataValue::Null),
    );

    // 进程级 Bean 先绑定；同名的执行级 context bean 随后覆盖，保持 Java bindParam 语义。
    for (bean_name, proxy) in ScriptBeanManager::get_script_bean_map() {
        values.insert(bean_name, script_bean_value(proxy));
    }
    if let Some(script) = cache.script.as_deref() {
        for bean_name in member_receiver_names(script) {
            if let Some(proxy) = context.bean::<ScriptBeanProxy>(&bean_name) {
                values.insert(bean_name, script_bean_value(proxy));
            }
        }
    }
    values
}

fn script_bean_value(proxy: Arc<ScriptBeanProxy>) -> DataValue {
    DataValue::Object(Rc::new(RefCell::new(ScriptBeanNativeObject { proxy })))
}

fn hash_map_to_index_map(values: HashMap<String, DataValue>) -> IndexMap {
    IndexMap::from_entries(
        values
            .into_iter()
            .map(|(key, value)| (DataValue::Str(key), value))
            .collect(),
    )
}

fn normalize_liteflow_calls(script: &str) -> String {
    let mut source = [
        ("defaultContext.getData", GET_DATA_FUNCTION),
        ("defaultContext.hasData", HAS_DATA_FUNCTION),
        ("defaultContext.setData", SET_DATA_FUNCTION),
        ("_meta.get", META_FUNCTION),
        ("System.out.println", PRINTLN_FUNCTION),
    ]
    .into_iter()
    .fold(script.to_string(), |source, (target, replacement)| {
        replace_call_target(&source, target, replacement)
    });
    for (bean_name, proxy) in ScriptBeanManager::get_script_bean_map() {
        for method_name in proxy.method_names() {
            source = replace_call_target(
                &source,
                &format!("{bean_name}.{method_name}"),
                &script_bean_function_name(&bean_name, &method_name),
            );
        }
    }
    // LiteFlow Java QLExpress 历史脚本接受 `not`，当前 Rust QLExpress 的
    // 一元否定词法记号为 `!`；这里只做词法别名，不参与表达式求值。
    replace_keyword(&source, "not", "!")
}

fn script_bean_function_name(bean_name: &str, method_name: &str) -> String {
    let encode = |value: &str| {
        value
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    };
    format!(
        "__liteflow_bean_{}_{}",
        encode(bean_name),
        encode(method_name)
    )
}

fn replace_keyword(source: &str, keyword: &str, replacement: &str) -> String {
    let bytes = source.as_bytes();
    let mut output = String::with_capacity(source.len());
    let mut index = 0;
    let mut quote = None;
    while index < bytes.len() {
        let character = source[index..]
            .chars()
            .next()
            .expect("index is within source");
        let character_len = character.len_utf8();
        if let Some(active_quote) = quote {
            output.push(character);
            index += character_len;
            if character == '\\' {
                if index < bytes.len() {
                    let escaped = source[index..]
                        .chars()
                        .next()
                        .expect("index is within source");
                    output.push(escaped);
                    index += escaped.len_utf8();
                }
            } else if character == active_quote {
                quote = None;
            }
            continue;
        }
        if character == '"' || character == '\'' {
            quote = Some(character);
            output.push(character);
            index += character_len;
            continue;
        }
        if source[index..].starts_with(keyword)
            && is_identifier_boundary(bytes.get(index.wrapping_sub(1)).copied())
            && is_identifier_boundary(bytes.get(index + keyword.len()).copied())
        {
            output.push_str(replacement);
            index += keyword.len();
            continue;
        }
        output.push(character);
        index += character_len;
    }
    output
}

fn replace_call_target(source: &str, target: &str, replacement: &str) -> String {
    let bytes = source.as_bytes();
    let mut output = String::with_capacity(source.len());
    let mut index = 0;
    let mut quote = None;
    let mut line_comment = false;
    let mut block_comment = false;
    while index < bytes.len() {
        let character = source[index..]
            .chars()
            .next()
            .expect("index is within source");
        let character_len = character.len_utf8();
        if line_comment {
            output.push(character);
            index += character_len;
            if character == '\n' {
                line_comment = false;
            }
            continue;
        }
        if block_comment {
            output.push(character);
            index += character_len;
            if character == '*' && bytes.get(index) == Some(&b'/') {
                output.push('/');
                index += 1;
                block_comment = false;
            }
            continue;
        }
        if let Some(active_quote) = quote {
            output.push(character);
            index += character_len;
            if character == '\\' {
                if index < bytes.len() {
                    let escaped = source[index..]
                        .chars()
                        .next()
                        .expect("index is within source");
                    output.push(escaped);
                    index += escaped.len_utf8();
                }
            } else if character == active_quote {
                quote = None;
            }
            continue;
        }
        if character == '"' || character == '\'' {
            quote = Some(character);
            output.push(character);
            index += character_len;
            continue;
        }
        if character == '/' && bytes.get(index + 1) == Some(&b'/') {
            output.push_str("//");
            index += 2;
            line_comment = true;
            continue;
        }
        if character == '/' && bytes.get(index + 1) == Some(&b'*') {
            output.push_str("/*");
            index += 2;
            block_comment = true;
            continue;
        }
        if source[index..].starts_with(target)
            && is_identifier_boundary(bytes.get(index.wrapping_sub(1)).copied())
            && is_call_suffix(source, index + target.len())
        {
            output.push_str(replacement);
            index += target.len();
            continue;
        }
        output.push(character);
        index += character_len;
    }
    output
}

fn is_identifier_boundary(character: Option<u8>) -> bool {
    character.is_none_or(|character| {
        !(character == b'_' || character.is_ascii_alphanumeric() || character == b'.')
    })
}

fn is_call_suffix(source: &str, mut index: usize) -> bool {
    while source
        .as_bytes()
        .get(index)
        .is_some_and(u8::is_ascii_whitespace)
    {
        index += 1;
    }
    source.as_bytes().get(index) == Some(&b'(')
}

fn member_receiver_names(source: &str) -> Vec<String> {
    let bytes = source.as_bytes();
    let mut names = Vec::new();
    let mut index = 0;
    let mut quote = None;
    while index < bytes.len() {
        let character = bytes[index] as char;
        if let Some(active_quote) = quote {
            if character == '\\' {
                index += 2;
                continue;
            }
            if character == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }
        if character == '"' || character == '\'' {
            quote = Some(character);
            index += 1;
            continue;
        }
        if !(character == '_' || character.is_ascii_alphabetic()) {
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        while bytes
            .get(index)
            .is_some_and(|byte| *byte == b'_' || byte.is_ascii_alphanumeric())
        {
            index += 1;
        }
        let end = index;
        if bytes.get(index) != Some(&b'.') {
            continue;
        }
        index += 1;
        if !bytes
            .get(index)
            .is_some_and(|byte| *byte == b'_' || byte.is_ascii_alphabetic())
        {
            continue;
        }
        while bytes
            .get(index)
            .is_some_and(|byte| *byte == b'_' || byte.is_ascii_alphanumeric())
        {
            index += 1;
        }
        while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
            index += 1;
        }
        if bytes.get(index) == Some(&b'(') {
            let name = source[start..end].to_string();
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names
}

fn json_to_data_value(value: &Value) -> Result<DataValue, String> {
    match value {
        Value::Null => Ok(DataValue::Null),
        Value::Bool(value) => Ok(DataValue::Bool(*value)),
        Value::Number(value) => number_to_data_value(value),
        Value::String(value) => Ok(DataValue::Str(value.clone())),
        Value::Array(values) => values
            .iter()
            .map(json_to_data_value)
            .collect::<Result<Vec<_>, _>>()
            .map(DataValue::list),
        Value::Object(values) => values
            .iter()
            .map(|(key, value)| Ok((DataValue::Str(key.clone()), json_to_data_value(value)?)))
            .collect::<Result<Vec<_>, String>>()
            .map(IndexMap::from_entries)
            .map(DataValue::map),
    }
}

fn number_to_data_value(value: &Number) -> Result<DataValue, String> {
    if let Some(value) = value.as_i64() {
        return Ok(i32::try_from(value)
            .map(DataValue::Int)
            .unwrap_or_else(|_| DataValue::Long(value)));
    }
    if let Some(value) = value.as_u64() {
        return i64::try_from(value)
            .map(DataValue::Long)
            .map_err(|_| format!("JSON unsigned integer [{value}] exceeds QLExpress Long"));
    }
    value
        .as_f64()
        .map(DataValue::Double)
        .ok_or_else(|| format!("unsupported JSON number [{value}]"))
}

fn data_value_to_json(value: &DataValue) -> Result<Value, String> {
    match value {
        DataValue::Null => Ok(Value::Null),
        DataValue::Bool(value) => Ok(Value::Bool(*value)),
        DataValue::Byte(value) => Ok(json!(*value)),
        DataValue::Short(value) => Ok(json!(*value)),
        DataValue::Int(value) => Ok(json!(*value)),
        DataValue::Long(value) => Ok(json!(*value)),
        DataValue::Float(value) => Number::from_f64(f64::from(*value))
            .map(Value::Number)
            .ok_or_else(|| "QLExpress Float result is not finite".to_string()),
        DataValue::Double(value) => Number::from_f64(*value)
            .map(Value::Number)
            .ok_or_else(|| "QLExpress Double result is not finite".to_string()),
        DataValue::BigInt(value) => value
            .to_string()
            .parse::<Number>()
            .map(Value::Number)
            .map_err(|error| format!("cannot convert QLExpress BigInteger: {error}")),
        DataValue::BigDec(value) => value
            .parse::<Number>()
            .map(Value::Number)
            .map_err(|error| format!("cannot convert QLExpress BigDecimal: {error}")),
        DataValue::Char(value) => Ok(Value::String(value.to_string())),
        DataValue::Str(value) => Ok(Value::String(value.clone())),
        DataValue::List(values) | DataValue::Array(values) => values
            .borrow()
            .iter()
            .map(data_value_to_json)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        DataValue::Map(values) => {
            let mut object = Map::new();
            for (key, value) in values.borrow().entries() {
                let DataValue::Str(key) = key else {
                    return Err(format!(
                        "QLExpress Map key [{}] cannot be represented as JSON object key",
                        key.string_value_of()
                    ));
                };
                object.insert(key.clone(), data_value_to_json(value)?);
            }
            Ok(Value::Object(object))
        }
        DataValue::Lambda(_) => {
            Err("QLExpress Lambda result cannot be converted to JSON".to_string())
        }
        DataValue::Object(_) => {
            Err("QLExpress native object result cannot be converted to JSON".to_string())
        }
    }
}

fn meta_value(context: &CmpContext, key: &str) -> Value {
    match key {
        "nodeId" => json!(context.node_id()),
        "tag" => context.tag().map_or(Value::Null, |tag| json!(tag)),
        "cmpData" => context
            .cmp_data()
            .and_then(|value| serde_json::from_str(value).ok())
            .unwrap_or_else(|| context.cmp_data().map_or(Value::Null, |value| json!(value))),
        "loopIndex" => context
            .loop_index()
            .map_or(Value::Null, |loop_index| json!(loop_index)),
        "loopObject" => context.loop_object::<Value>().unwrap_or(Value::Null),
        "requestData" => context.request_data::<Value>().unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

fn shared_executor() -> Arc<QlExpressScriptExecutor> {
    static EXECUTOR: OnceLock<Arc<QlExpressScriptExecutor>> = OnceLock::new();
    Arc::clone(EXECUTOR.get_or_init(|| Arc::new(QlExpressScriptExecutor::new())))
}

fn cache_error(operation: &str) -> LiteflowError {
    LiteflowError::Script {
        node: String::new(),
        msg: format!("qlexpress script cache {operation} lock poisoned"),
    }
}

fn script_error(node_id: &str, error: QLException) -> LiteflowError {
    LiteflowError::Script {
        node: node_id.to_string(),
        msg: format!(
            "QLExpress {} [{}] at {}:{}: {}",
            if error.is_syntax() {
                "syntax error"
            } else {
                "runtime error"
            },
            error.error_code(),
            error.line_no(),
            error.col_no(),
            error
        ),
    }
}

fn ql_bridge_error(message: impl Into<String>) -> QLException {
    QLException::for_test(
        QLExceptionKind::Runtime,
        message.into(),
        "LITEFLOW_SCRIPT_BRIDGE_ERROR",
    )
}

#[cfg(test)]
mod tests {
    use super::{QlExpressScriptExecutor, member_receiver_names, normalize_liteflow_calls};
    use liteflow_core::script::ScriptExecutor;

    #[test]
    fn normalizes_only_executable_liteflow_calls() {
        let source = r#"
            // defaultContext.getData("ignored")
            text = "defaultContext.setData(\"ignored\", 1)";
            value = defaultContext.getData("score");
            defaultContext.setData("answer", value);
        "#;
        let normalized = normalize_liteflow_calls(source);
        assert!(normalized.contains(r#"// defaultContext.getData("ignored")"#));
        assert!(normalized.contains(r#""defaultContext.setData(\"ignored\", 1)""#));
        assert!(normalized.contains(r#"__liteflow_get_data("score")"#));
        assert!(normalized.contains(r#"__liteflow_set_data("answer", value)"#));
    }

    #[test]
    fn finds_dynamic_script_bean_receivers() {
        assert_eq!(
            member_receiver_names(
                "order.setOrderType(6); ql_math.double(21); order.getOrderType()"
            ),
            vec!["order".to_string(), "ql_math".to_string()]
        );
    }

    #[test]
    fn java_compile_and_cache_aliases_use_the_published_qlexpress_runtime() {
        let executor = QlExpressScriptExecutor::new();

        <QlExpressScriptExecutor as ScriptExecutor>::compile(&executor, "1 + 2").unwrap();
        assert!(<QlExpressScriptExecutor as ScriptExecutor>::compile(&executor, "1 +").is_err());

        executor.load("ql-node", "40 + 2").unwrap();
        assert_eq!(executor.get_node_ids().unwrap(), vec!["ql-node"]);
        executor.un_load("ql-node").unwrap();
        assert!(executor.get_node_ids().unwrap().is_empty());
    }
}
