//! 对应 Java: `com.yomahub.liteflow.script.kotlin.KotlinScriptExecutor`。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use liteflow_core::LFResult;
use liteflow_core::core::NodeComponent;
use liteflow_core::script::{ScriptExecutorFactory, ScriptKind, build_rhai_component};

/// Kotlin LiteFlow 常用绑定语义的 Rust 受控执行器。
///
/// Java 版本通过 JSR223 Kotlin 引擎编译脚本；Rust 版本保留 `val`/`var`、显式类型、
/// 表达式函数与块函数、`String.toInt()`、DefaultContext、`bindings`、ScriptBean
/// 与 `_meta` 的可验证语义，并编译为受控 Rhai AST。JVM classpath 中任意类的
/// 实例化仍不开放，但仅用于声明类型的 `import` 会被安全消解。
///
/// 对应 Java: `com.yomahub.liteflow.script.kotlin.KotlinScriptExecutor`。
pub struct KotlinScriptExecutor;

impl KotlinScriptExecutor {
    /// 注册 `language = "kotlin"` 的组件构建器。
    ///
    /// 对应 Java: `KotlinScriptExecutor#scriptType` 与 ServiceLoader 注册。
    pub fn register() -> LFResult<()> {
        ScriptExecutorFactory::register("kotlin", Self::build)
    }

    fn build(node_id: &str, kind: ScriptKind, script: &str) -> LFResult<Arc<dyn NodeComponent>> {
        let normalized = normalize_script(node_id, kind, script)?;
        build_rhai_component(node_id, kind, &normalized)
    }
}

/// 将 Java testcase 中的 Kotlin 基线语法转换为 Rust 受控脚本。
///
/// `val` 在编译期禁止再次赋值，`var` 保留可变语义；用户函数获得请求级数据和
/// ScriptBean 隐式桥参数，因此块函数内的 `bindings["defaultContext"]` 与 Java
/// 一样操作当前 Slot，而不是复制一份临时 Map。对应 Java:
/// `JSR223ScriptExecutor#compile`、`ScriptExecutor#bindParam` 和 Kotlin 编译器。
fn normalize_script(node_id: &str, kind: ScriptKind, script: &str) -> LFResult<String> {
    let (functions, top_level_lines) = extract_functions(node_id, script)?;
    let function_names = functions
        .iter()
        .map(|function| function.name.clone())
        .collect::<HashSet<_>>();
    let mut normalized = Vec::new();

    // 函数先生成，保证 Rhai 编译器可以解析顶层调用；每个函数只在自己的局部状态
    // 中执行 val/var 检查，不会把同名局部变量污染到其他函数或顶层脚本。
    for function in functions {
        normalized.push(normalize_function(node_id, function, &function_names)?);
    }

    let mut state = NormalizeState::default();
    let top_level_count = top_level_lines.len();
    for (index, line) in top_level_lines.into_iter().enumerate() {
        let mut statements = normalize_statement(node_id, &line, &function_names, &mut state)?;
        if kind != ScriptKind::Common
            && index + 1 == top_level_count
            && statements.len() == 1
            && !statements[0].trim_start().starts_with("return ")
            && parse_declaration(&line).is_none()
            && assignment_variable(&line).is_none()
        {
            let expression = statements[0].trim().trim_end_matches(';');
            statements[0] = format!("return {expression};");
        }
        normalized.extend(statements);
    }

    if normalized.iter().all(|line| line.trim().is_empty()) {
        return Err(script_error(node_id, "Kotlin script cannot be empty"));
    }
    Ok(normalized.join("\n"))
}

#[derive(Default)]
struct NormalizeState {
    immutable_variables: HashSet<String>,
    mutable_variables: HashSet<String>,
    context_aliases: HashSet<String>,
    meta_aliases: HashSet<String>,
    bean_aliases: HashMap<String, String>,
}

struct KotlinFunction {
    name: String,
    parameters: Vec<KotlinParameter>,
    body: Vec<String>,
    expression_body: Option<String>,
}

struct KotlinParameter {
    name: String,
    declared_type: Option<String>,
}

fn extract_functions(node_id: &str, script: &str) -> LFResult<(Vec<KotlinFunction>, Vec<String>)> {
    let lines = script.lines().collect::<Vec<_>>();
    let mut functions = Vec::new();
    let mut top_level = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let source = strip_line_comment(lines[index]).trim();
        if source.is_empty() || source.starts_with("import ") {
            index += 1;
            continue;
        }
        if source.starts_with("package ")
            || source.starts_with("class ")
            || source.starts_with("object ")
        {
            return Err(script_error(
                node_id,
                "Kotlin package/class/object declaration is outside the Rust controlled runtime",
            ));
        }
        if !source.starts_with("fun ") {
            top_level.push(source.to_string());
            index += 1;
            continue;
        }

        let (name, parameters, tail) = parse_function_header(node_id, source)?;
        if let Some(expression) = tail.strip_prefix('=').map(str::trim) {
            functions.push(KotlinFunction {
                name,
                parameters,
                body: Vec::new(),
                expression_body: Some(expression.to_string()),
            });
            index += 1;
            continue;
        }
        if !tail.ends_with('{') && tail != "{" {
            return Err(script_error(
                node_id,
                format!("Kotlin function [{name}] must use an expression body or a block"),
            ));
        }

        let mut body = Vec::new();
        let mut brace_depth = brace_delta(source);
        index += 1;
        while index < lines.len() && brace_depth > 0 {
            let line = strip_line_comment(lines[index]).trim();
            let delta = brace_delta(line);
            brace_depth += delta;
            if !(brace_depth == 0 && line == "}") && !line.is_empty() {
                body.push(line.to_string());
            }
            index += 1;
        }
        if brace_depth != 0 {
            return Err(script_error(
                node_id,
                format!("unclosed Kotlin function [{name}]"),
            ));
        }
        functions.push(KotlinFunction {
            name,
            parameters,
            body,
            expression_body: None,
        });
    }
    Ok((functions, top_level))
}

fn parse_function_header(
    node_id: &str,
    source: &str,
) -> LFResult<(String, Vec<KotlinParameter>, String)> {
    let declaration = source
        .strip_prefix("fun ")
        .ok_or_else(|| script_error(node_id, "invalid Kotlin function declaration"))?;
    let open = declaration
        .find('(')
        .ok_or_else(|| script_error(node_id, "Kotlin function is missing parameter list"))?;
    let close = matching_parenthesis(declaration, open)
        .ok_or_else(|| script_error(node_id, "unclosed Kotlin function parameter list"))?;
    let name = declaration[..open].trim();
    if !is_identifier(name) {
        return Err(script_error(
            node_id,
            format!("invalid Kotlin function name [{name}]"),
        ));
    }
    let parameters = split_arguments(&declaration[open + 1..close])
        .into_iter()
        .filter(|parameter| !parameter.trim().is_empty())
        .map(|parameter| {
            let (name, declared_type) = parameter
                .split_once(':')
                .map_or((parameter.trim(), None), |(name, ty)| {
                    (name.trim(), Some(ty.trim().to_string()))
                });
            if !is_identifier(name) {
                return Err(script_error(
                    node_id,
                    format!("invalid Kotlin parameter name [{name}]"),
                ));
            }
            Ok(KotlinParameter {
                name: name.to_string(),
                declared_type,
            })
        })
        .collect::<LFResult<Vec<_>>>()?;
    let mut tail = declaration[close + 1..].trim();
    if let Some(after_type) = tail.strip_prefix(':') {
        tail = after_type
            .find(['=', '{'])
            .map_or("", |position| &after_type[position..]);
    }
    Ok((name.to_string(), parameters, tail.to_string()))
}

fn normalize_function(
    node_id: &str,
    function: KotlinFunction,
    function_names: &HashSet<String>,
) -> LFResult<String> {
    let mut state = NormalizeState::default();
    for parameter in &function.parameters {
        state.mutable_variables.insert(parameter.name.clone());
        if parameter.declared_type.as_deref() == Some("DefaultContext") {
            state.context_aliases.insert(parameter.name.clone());
        }
    }
    let mut body = Vec::new();
    if let Some(expression) = function.expression_body {
        let expression = normalize_expression(node_id, &expression, function_names, &state)?;
        body.push(expression);
    } else {
        for line in function.body {
            body.extend(normalize_statement(
                node_id,
                &line,
                function_names,
                &mut state,
            )?);
        }
    }
    let mut parameters = function
        .parameters
        .iter()
        .map(|parameter| parameter.name.clone())
        .collect::<Vec<_>>();
    parameters.extend(["_script_data".to_string(), "_script_beans".to_string()]);
    Ok(format!(
        "fn {}({}) {{\n{}\n}}",
        function.name,
        parameters.join(", "),
        body.join("\n")
    ))
}

fn normalize_statement(
    node_id: &str,
    source: &str,
    function_names: &HashSet<String>,
    state: &mut NormalizeState,
) -> LFResult<Vec<String>> {
    let source = strip_line_comment(source).trim();
    if source.is_empty() || source.starts_with("import ") {
        return Ok(Vec::new());
    }
    let line = normalize_safe_context_receiver(source);

    if let Some(statement) =
        normalize_control_flow_statement(node_id, &line, function_names, state)?
    {
        return Ok(vec![statement]);
    }

    if let Some(statements) = normalize_prefix_decrement_set_data(node_id, &line, state)? {
        return Ok(statements);
    }

    if let Some(declaration) = parse_declaration(&line) {
        if let Some(binding_name) = binding_name(declaration.expression) {
            if binding_name == "defaultContext" {
                state.context_aliases.insert(declaration.name.to_string());
            } else if binding_name == "_meta" {
                state.meta_aliases.insert(declaration.name.to_string());
            } else {
                state
                    .bean_aliases
                    .insert(declaration.name.to_string(), binding_name.to_string());
            }
            if declaration.mutable {
                state.mutable_variables.insert(declaration.name.to_string());
            } else {
                state
                    .immutable_variables
                    .insert(declaration.name.to_string());
            }
            return Ok(Vec::new());
        }

        validate_declared_type(node_id, declaration.declared_type, declaration.expression)?;
        let expression =
            normalize_expression(node_id, declaration.expression, function_names, state)?;
        if declaration.mutable {
            state.mutable_variables.insert(declaration.name.to_string());
        } else {
            state
                .immutable_variables
                .insert(declaration.name.to_string());
        }
        return Ok(vec![terminate_statement(format!(
            "let {} = {expression}",
            declaration.name
        ))]);
    }

    if let Some(expression) = line.strip_prefix("return ") {
        return Ok(vec![terminate_statement(format!(
            "return {}",
            normalize_expression(node_id, expression, function_names, state)?
        ))]);
    }

    if let Some(statement) = normalize_throw_statement(node_id, &line, function_names, state)? {
        return Ok(vec![statement]);
    }

    if let Some(variable) = assignment_variable(&line) {
        if state.immutable_variables.contains(variable) {
            return Err(script_error(
                node_id,
                format!("Kotlin val [{variable}] cannot be reassigned"),
            ));
        }
        if !state.mutable_variables.contains(variable) {
            return Err(script_error(
                node_id,
                format!("Kotlin variable [{variable}] must be declared with val or var"),
            ));
        }
    }
    let expression = normalize_expression(node_id, &line, function_names, state)?;
    Ok(vec![terminate_statement(expression)])
}

/// 保留 Kotlin 块级 `if/else if/else` 的控制流边界。
///
/// 普通表达式需要补分号，而控制块头和闭合花括号不能补分号；条件表达式仍经过
/// bindings、DefaultContext、ScriptBean 与用户函数调用转换。对应 Java testcase:
/// `LiteflowKotlinScriptRefreshELTest#testRefresh1` 中 `getId` 的路由判断。
fn normalize_control_flow_statement(
    node_id: &str,
    source: &str,
    function_names: &HashSet<String>,
    state: &NormalizeState,
) -> LFResult<Option<String>> {
    let source = source.trim().trim_end_matches(';').trim();
    let compact = source
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    match compact.as_str() {
        "}" => return Ok(Some("}".to_string())),
        "}else{" => return Ok(Some("} else {".to_string())),
        "else{" => return Ok(Some("else {".to_string())),
        _ => {}
    }

    let (prefix, if_source) = if let Some(value) = source.strip_prefix("} else ") {
        ("} else ", value)
    } else if let Some(value) = source.strip_prefix("else ") {
        ("else ", value)
    } else {
        ("", source)
    };
    let Some(after_if) = if_source.strip_prefix("if") else {
        return Ok(None);
    };
    let after_if = after_if.trim_start();
    if !after_if.starts_with('(') {
        return Ok(None);
    }
    let open = if_source
        .find('(')
        .ok_or_else(|| script_error(node_id, "Kotlin if is missing condition"))?;
    let close = matching_parenthesis(if_source, open)
        .ok_or_else(|| script_error(node_id, "unclosed Kotlin if condition"))?;
    if if_source[close + 1..].trim() != "{" {
        return Err(script_error(
            node_id,
            "Kotlin controlled runtime requires braces around if/else bodies",
        ));
    }
    let condition =
        normalize_expression(node_id, &if_source[open + 1..close], function_names, state)?;
    Ok(Some(format!("{prefix}if ({condition}) {{")))
}

/// 将 Kotlin `throw LiteFlowException(...)` 构造映射为受控运行时业务异常。
///
/// Java testcase 使用自定义 `LiteFlowException` 子类并依赖 code/message 构造器；
/// Rust 不加载 JVM classpath，而是保留一参数 message 与二参数 code/message
/// 两种异常语义。对应 Java:
/// `ThrowExceptionScriptKotlinELTest#test1`。
fn normalize_throw_statement(
    node_id: &str,
    source: &str,
    function_names: &HashSet<String>,
    state: &NormalizeState,
) -> LFResult<Option<String>> {
    let Some(exception) = source.strip_prefix("throw ").map(str::trim) else {
        return Ok(None);
    };
    let open = exception
        .find('(')
        .ok_or_else(|| script_error(node_id, "Kotlin throw must construct an exception"))?;
    let close = matching_parenthesis(exception, open)
        .ok_or_else(|| script_error(node_id, "unclosed Kotlin exception constructor"))?;
    if !exception[close + 1..].trim().is_empty() {
        return Err(script_error(
            node_id,
            "unexpected content after Kotlin exception constructor",
        ));
    }
    let exception_type = exception[..open].trim();
    if !exception_type.split('.').all(is_identifier) {
        return Err(script_error(
            node_id,
            format!("invalid Kotlin exception type [{exception_type}]"),
        ));
    }
    let arguments = split_arguments(&exception[open + 1..close]);
    if !matches!(arguments.len(), 1 | 2) {
        return Err(script_error(
            node_id,
            format!(
                "Kotlin LiteFlow exception [{exception_type}] requires message or code/message"
            ),
        ));
    }
    let arguments = arguments
        .into_iter()
        .map(|argument| normalize_expression(node_id, argument, function_names, state))
        .collect::<LFResult<Vec<_>>>()?;
    Ok(Some(terminate_statement(format!(
        "liteflow_throw({})",
        arguments.join(", ")
    ))))
}

fn normalize_prefix_decrement_set_data(
    node_id: &str,
    source: &str,
    state: &NormalizeState,
) -> LFResult<Option<Vec<String>>> {
    let mut aliases = state.context_aliases.clone();
    aliases.insert("defaultContext".to_string());
    for alias in aliases {
        let prefix = format!("{alias}.setData(");
        let Some(arguments) = source
            .strip_prefix(&prefix)
            .and_then(|value| value.trim_end_matches(';').strip_suffix(')'))
        else {
            continue;
        };
        let arguments = split_arguments(arguments);
        if arguments.len() != 2 {
            return Err(script_error(
                node_id,
                format!("{alias}.setData requires 2 arguments"),
            ));
        }
        let Some(variable) = arguments[1].trim().strip_prefix("--").map(str::trim) else {
            return Ok(None);
        };
        if !state.mutable_variables.contains(variable) {
            return Err(script_error(
                node_id,
                format!("Kotlin prefix decrement requires mutable var [{variable}]"),
            ));
        }
        return Ok(Some(vec![
            format!("{variable} -= 1;"),
            format!(
                "script_data_set(_script_data, {}, {variable});",
                arguments[0].trim()
            ),
        ]));
    }
    Ok(None)
}

struct Declaration<'a> {
    mutable: bool,
    name: &'a str,
    declared_type: Option<&'a str>,
    expression: &'a str,
}

fn parse_declaration(statement: &str) -> Option<Declaration<'_>> {
    let (mutable, rest) = statement
        .strip_prefix("val ")
        .map(|rest| (false, rest))
        .or_else(|| statement.strip_prefix("var ").map(|rest| (true, rest)))?;
    let (left, expression) = rest.split_once('=')?;
    let (name, declared_type) = left
        .split_once(':')
        .map_or((left.trim(), None), |(name, ty)| {
            (name.trim(), Some(ty.trim()))
        });
    is_identifier(name).then_some(Declaration {
        mutable,
        name,
        declared_type,
        expression: expression.trim(),
    })
}

fn validate_declared_type(
    node_id: &str,
    declared_type: Option<&str>,
    expression: &str,
) -> LFResult<()> {
    let Some(declared_type) = declared_type else {
        return Ok(());
    };
    let converted_to_int = expression.ends_with(".toInt()") || expression.ends_with(".toLong()");
    let quoted = is_quoted(expression);
    let numeric = expression.parse::<f64>().is_ok();
    let boolean = matches!(expression, "true" | "false");
    let mismatch = match declared_type {
        "Int" | "Long" => quoted && !converted_to_int || boolean,
        "Double" | "Float" => quoted || boolean,
        "Boolean" => quoted || numeric,
        "String" => numeric || boolean,
        _ => {
            return Err(script_error(
                node_id,
                format!("unsupported Kotlin declared type [{declared_type}]"),
            ));
        }
    };
    if mismatch {
        Err(script_error(
            node_id,
            format!(
                "Kotlin type mismatch: expression [{expression}] cannot be assigned to [{declared_type}]"
            ),
        ))
    } else {
        Ok(())
    }
}

fn normalize_conversion(expression: &str) -> String {
    for suffix in [".toInt()", ".toLong()"] {
        if let Some(value) = expression.strip_suffix(suffix) {
            return format!("kotlin_to_int({})", value.trim());
        }
    }
    expression.to_string()
}

fn normalize_expression(
    node_id: &str,
    expression: &str,
    function_names: &HashSet<String>,
    state: &NormalizeState,
) -> LFResult<String> {
    let mut source = expression.trim().trim_end_matches(';').to_string();
    source = source
        .replace("println(", "print(")
        .replace("_meta.requestData", "input")
        .replace("_meta.cmpData", "cmp_data")
        .replace("_meta.loopObject", "loop_object")
        .replace("_meta.loopIndex", "loop_index")
        .replace("_meta.nodeId", "node_id")
        .replace("_meta.tag", "tag");

    // Kotlin 的显式强制转换只承担编译期类型约束；绑定对象在 Rust 中已经由
    // ScriptDataBindings 或 ScriptBeanProxy 强类型桥接，因此不保留 JVM cast。
    source = strip_kotlin_cast(&source);
    source = replace_meta_aliases(source, &state.meta_aliases);
    source = replace_context_alias_calls(node_id, source, &state.context_aliases)?;
    source = replace_bean_alias_calls(node_id, source, &state.bean_aliases)?;
    source = replace_direct_binding(node_id, source)?;
    source = replace_function_calls(node_id, source, function_names)?;
    Ok(normalize_conversion(&source))
}

fn replace_context_alias_calls(
    node_id: &str,
    mut source: String,
    aliases: &HashSet<String>,
) -> LFResult<String> {
    let mut all_aliases = aliases.clone();
    all_aliases.insert("defaultContext".to_string());
    for alias in all_aliases {
        for (method, expected_arguments, target) in [
            ("getData", 1, "script_data_get"),
            ("hasData", 1, "script_data_has"),
            ("setData", 2, "script_data_set"),
        ] {
            let needle = format!("{alias}.{method}(");
            while let Some(start) = find_identifier_call(&source, &needle) {
                let open = start + needle.len() - 1;
                let close = matching_parenthesis(&source, open).ok_or_else(|| {
                    script_error(node_id, format!("unclosed {alias}.{method} call"))
                })?;
                let arguments = split_arguments(&source[open + 1..close]);
                if arguments.len() != expected_arguments {
                    return Err(script_error(
                        node_id,
                        format!(
                            "{alias}.{method} requires {expected_arguments} argument(s), got {}",
                            arguments.len()
                        ),
                    ));
                }
                let replacement = format!(
                    "{target}(_script_data, {})",
                    arguments
                        .iter()
                        .map(|argument| argument.trim())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                source.replace_range(start..=close, &replacement);
            }
        }
    }
    Ok(source)
}

fn replace_bean_alias_calls(
    node_id: &str,
    mut source: String,
    aliases: &HashMap<String, String>,
) -> LFResult<String> {
    for (alias, binding_name) in aliases {
        let alias_prefix = format!("{alias}.");
        let mut search_from = 0;
        while let Some(relative_dot) = source[search_from..].find(&alias_prefix) {
            let start = search_from + relative_dot;
            if start > 0
                && source[..start]
                    .chars()
                    .next_back()
                    .is_some_and(|character| character == '_' || character.is_ascii_alphanumeric())
            {
                search_from = start + alias.len() + 1;
                continue;
            }
            let method_start = start + alias.len() + 1;
            let Some(relative_open) = source[method_start..].find('(') else {
                break;
            };
            let open = method_start + relative_open;
            let method = source[method_start..open].trim();
            if !is_identifier(method) {
                search_from = open + 1;
                continue;
            }
            let close = matching_parenthesis(&source, open).ok_or_else(|| {
                script_error(
                    node_id,
                    format!("unclosed Kotlin ScriptBean call [{alias}.{method}]"),
                )
            })?;
            let arguments = split_arguments(&source[open + 1..close]);
            let argument_array = if arguments.len() == 1 && arguments[0].trim().is_empty() {
                "[]".to_string()
            } else {
                format!(
                    "[{}]",
                    arguments
                        .iter()
                        .map(|argument| argument.trim())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            let replacement = format!(
                "script_context_call(_script_beans, {binding_name:?}, {method:?}, {argument_array})"
            );
            source.replace_range(start..=close, &replacement);
            search_from = start + replacement.len();
        }
    }
    Ok(source)
}

fn replace_function_calls(
    node_id: &str,
    mut source: String,
    function_names: &HashSet<String>,
) -> LFResult<String> {
    let mut names = function_names.iter().collect::<Vec<_>>();
    names.sort_by_key(|name| std::cmp::Reverse(name.len()));
    for name in names {
        let needle = format!("{name}(");
        let mut search_from = 0;
        while let Some(relative_start) = source[search_from..].find(&needle) {
            let start = search_from + relative_start;
            if start > 0
                && source[..start]
                    .chars()
                    .next_back()
                    .is_some_and(|character| character == '_' || character.is_ascii_alphanumeric())
            {
                search_from = start + needle.len();
                continue;
            }
            let open = start + name.len();
            let close = matching_parenthesis(&source, open).ok_or_else(|| {
                script_error(node_id, format!("unclosed Kotlin function call [{name}]"))
            })?;
            let current_arguments = source[open + 1..close].trim();
            let hidden_arguments = if current_arguments.is_empty() {
                "_script_data, _script_beans".to_string()
            } else {
                format!("{current_arguments}, _script_data, _script_beans")
            };
            source.replace_range(open + 1..close, &hidden_arguments);
            search_from = open + hidden_arguments.len() + 2;
        }
    }
    Ok(source)
}

fn replace_direct_binding(node_id: &str, mut source: String) -> LFResult<String> {
    while let Some(start) = source.find("bindings[") {
        let key_start = start + "bindings[".len();
        let close = source[key_start..]
            .find(']')
            .map(|relative| key_start + relative)
            .ok_or_else(|| script_error(node_id, "unclosed Kotlin bindings access"))?;
        let key = source[key_start..close].trim();
        let binding = key
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .or_else(|| {
                key.strip_prefix('\'')
                    .and_then(|value| value.strip_suffix('\''))
            })
            .ok_or_else(|| script_error(node_id, "Kotlin bindings key must be a string literal"))?;
        let replacement = match binding {
            "defaultContext" => "_script_data".to_string(),
            "_meta" => "_meta".to_string(),
            other => format!("{other:?}"),
        };
        let mut replace_end = close + 1;
        let suffix = &source[replace_end..];
        let cast_prefix_length = if suffix.trim_start().starts_with("as? ") {
            suffix.len() - suffix.trim_start().len() + "as? ".len()
        } else if suffix.trim_start().starts_with("as ") {
            suffix.len() - suffix.trim_start().len() + "as ".len()
        } else {
            0
        };
        if cast_prefix_length > 0 {
            replace_end += cast_prefix_length;
            while source[replace_end..]
                .chars()
                .next()
                .is_some_and(|character| {
                    character.is_ascii_alphanumeric()
                        || matches!(character, '_' | '<' | '>' | '*' | '?')
                })
            {
                replace_end += source[replace_end..]
                    .chars()
                    .next()
                    .map(char::len_utf8)
                    .unwrap_or_default();
            }
        }
        source.replace_range(start..replace_end, &replacement);
    }
    Ok(source)
}

fn replace_meta_aliases(mut source: String, aliases: &HashSet<String>) -> String {
    for alias in aliases {
        source = source.replace(&format!("{alias}["), "_meta[");
    }
    source
}

fn normalize_safe_context_receiver(source: &str) -> String {
    [
        r#"(bindings["defaultContext"] as? DefaultContext)?."#,
        r#"(bindings["defaultContext"] as DefaultContext)."#,
        r#"(bindings['defaultContext'] as? DefaultContext)?."#,
        r#"(bindings['defaultContext'] as DefaultContext)."#,
    ]
    .into_iter()
    .fold(source.to_string(), |value, pattern| {
        value.replace(pattern, "defaultContext.")
    })
}

fn binding_name(expression: &str) -> Option<&str> {
    let expression = expression.trim();
    let start = expression.find("bindings[")? + "bindings[".len();
    let close = expression[start..]
        .find(']')
        .map(|relative| start + relative)?;
    let key = expression[start..close].trim();
    key.strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            key.strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
}

fn strip_kotlin_cast(source: &str) -> String {
    let mut result = source.to_string();
    for marker in [" as? ", " as "] {
        if let Some(position) = result.rfind(marker) {
            let suffix = &result[position + marker.len()..];
            if suffix.chars().all(|character| {
                character.is_ascii_alphanumeric()
                    || matches!(character, '_' | '<' | '>' | ',' | ' ' | '*' | '?')
            }) {
                result.truncate(position);
            }
        }
    }
    result
}

fn find_identifier_call(source: &str, needle: &str) -> Option<usize> {
    let mut search_from = 0;
    while let Some(relative) = source[search_from..].find(needle) {
        let start = search_from + relative;
        if start == 0
            || source[..start]
                .chars()
                .next_back()
                .is_none_or(|character| character != '_' && !character.is_ascii_alphanumeric())
        {
            return Some(start);
        }
        search_from = start + needle.len();
    }
    None
}

fn strip_line_comment(source: &str) -> &str {
    let mut quote = None;
    let mut escaped = false;
    let characters = source.char_indices().collect::<Vec<_>>();
    for window in characters.windows(2) {
        let (index, character) = window[0];
        let (_, next) = window[1];
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == active_quote {
                quote = None;
            }
            continue;
        }
        if matches!(character, '"' | '\'') {
            quote = Some(character);
        } else if character == '/' && next == '/' {
            return &source[..index];
        }
    }
    source
}

fn brace_delta(source: &str) -> i32 {
    let mut delta = 0;
    let mut quote = None;
    let mut escaped = false;
    for character in source.chars() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == active_quote {
                quote = None;
            }
            continue;
        }
        match character {
            '"' | '\'' => quote = Some(character),
            '{' => delta += 1,
            '}' => delta -= 1,
            _ => {}
        }
    }
    delta
}

fn matching_parenthesis(source: &str, open: usize) -> Option<usize> {
    let mut depth = 0_u32;
    let mut quote = None;
    let mut escaped = false;
    for (relative, character) in source[open..].char_indices() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == active_quote {
                quote = None;
            }
            continue;
        }
        match character {
            '"' | '\'' => quote = Some(character),
            '(' => depth += 1,
            ')' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(open + relative);
                }
            }
            _ => {}
        }
    }
    None
}

fn split_arguments(arguments: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut depth = 0_u32;
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in arguments.char_indices() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == active_quote {
                quote = None;
            }
            continue;
        }
        match character {
            '"' | '\'' => quote = Some(character),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                result.push(&arguments[start..index]);
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    result.push(&arguments[start..]);
    result
}

fn assignment_variable(statement: &str) -> Option<&str> {
    let (candidate, operator_tail) = statement.split_once('=')?;
    if operator_tail.starts_with('=') || candidate.ends_with(['!', '<', '>']) {
        return None;
    }
    let candidate = candidate.trim();
    is_identifier(candidate).then_some(candidate)
}

fn is_identifier(value: &str) -> bool {
    value
        .chars()
        .next()
        .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
        && value
            .chars()
            .all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn is_quoted(value: &str) -> bool {
    value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
}

fn terminate_statement(statement: String) -> String {
    let trimmed = statement.trim_end();
    if trimmed.ends_with(';') || trimmed.ends_with('{') || trimmed.ends_with('}') {
        statement
    } else {
        format!("{trimmed};")
    }
}

fn script_error(node_id: &str, message: impl Into<String>) -> liteflow_core::LiteflowError {
    liteflow_core::LiteflowError::Script {
        node: node_id.to_string(),
        msg: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use liteflow_core::script::ScriptKind;

    use super::normalize_script;

    #[test]
    fn validates_kotlin_typed_declarations_and_conversion() {
        let normalized = normalize_script(
            "kotlin",
            ScriptKind::Common,
            r#"
                val number: Int = "123".toInt()
                var total: Int = 2
                total = total + number
            "#,
        )
        .unwrap();
        assert!(normalized.contains(r#"let number = kotlin_to_int("123");"#));
        assert!(normalized.contains("let total = 2;"));
        assert!(normalized.contains("total = total + number;"));
        assert!(
            normalize_script("kotlin", ScriptKind::Common, r#"val number: Int = "123""#).is_err()
        );
        assert!(
            normalize_script(
                "kotlin",
                ScriptKind::Common,
                "val number: Int = 1\nnumber = 2"
            )
            .is_err()
        );
    }

    #[test]
    fn normalizes_default_context_and_meta() {
        let normalized = normalize_script(
            "kotlin",
            ScriptKind::Common,
            r#"
                val score: Int = input.score
                defaultContext.setData("score", score)
                val node: String = _meta.nodeId
            "#,
        )
        .unwrap();
        assert!(normalized.contains("let score = input.score;"));
        assert!(
            normalized.contains(r#"script_data_set(_script_data, "score", score);"#),
            "{normalized}"
        );
        assert!(normalized.contains("let node = node_id;"));
    }

    #[test]
    fn normalizes_java_testcase_functions_and_bindings() {
        let normalized = normalize_script(
            "kotlin",
            ScriptKind::Common,
            r#"
                import com.yomahub.liteflow.slot.DefaultContext

                fun sum(a: Int, b: Int) = a + b
                var a = 2
                var b = 3
                val defaultContext = bindings["defaultContext"] as DefaultContext
                defaultContext.setData("s1", sum(a, b))
            "#,
        )
        .unwrap();
        assert!(
            normalized.contains("fn sum(a, b, _script_data, _script_beans)"),
            "{normalized}"
        );
        assert!(
            normalized.contains(
                r#"script_data_set(_script_data, "s1", sum(a, b, _script_data, _script_beans));"#
            ),
            "{normalized}"
        );
    }

    #[test]
    fn normalizes_block_function_return_for_typed_script_nodes() {
        let normalized = normalize_script(
            "kotlin",
            ScriptKind::For,
            r#"
                fun getCount(): Int {
                    val ctx = bindings["defaultContext"] as DefaultContext
                    var n1 = ctx.getData("k1") as Int
                    var n2 = ctx.getData("k2") as Int
                    return n1 + n2
                }
                getCount()
            "#,
        )
        .unwrap();
        assert!(
            normalized.contains(r#"let n1 = script_data_get(_script_data, "k1");"#),
            "{normalized}"
        );
        assert!(
            normalized.contains("return getCount(_script_data, _script_beans);"),
            "{normalized}"
        );
    }

    #[test]
    fn normalizes_liteflow_exception_code_and_message() {
        let normalized = normalize_script(
            "kotlin",
            ScriptKind::Common,
            r#"
                import com.example.TestException
                throw TestException("T01", "测试错误")
            "#,
        )
        .unwrap();
        assert!(
            normalized.contains(r#"liteflow_throw("T01", "测试错误");"#),
            "{normalized}"
        );
    }

    #[test]
    fn normalizes_refresh_switch_function_control_flow() {
        let normalized = normalize_script(
            "kotlin",
            ScriptKind::Switch,
            r#"
                import com.yomahub.liteflow.slot.DefaultContext

                fun getId(): String {
                    val context = bindings["defaultContext"] as DefaultContext
                    var count = context.getData("count") as Int
                    if(count > 100) {
                        return "pass"
                    } else {
                        return "fail"
                    }
                }
                getId()
            "#,
        )
        .unwrap();
        assert!(normalized.contains("if (count > 100) {"), "{normalized}");
        assert!(normalized.contains("} else {"), "{normalized}");
        assert!(
            normalized.contains("return getId(_script_data, _script_beans);"),
            "{normalized}"
        );
    }
}
