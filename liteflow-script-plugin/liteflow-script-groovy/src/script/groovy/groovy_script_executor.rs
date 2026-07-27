//! 对应 Java: `com.yomahub.liteflow.script.groovy.GroovyScriptExecutor`。

use std::collections::HashSet;
use std::sync::Arc;

use liteflow_core::LFResult;
use liteflow_core::core::NodeComponent;
use liteflow_core::script::{ScriptExecutorFactory, ScriptKind, build_rhai_component};

/// Groovy LiteFlow 绑定语义的 Rust 受控执行器。
pub struct GroovyScriptExecutor;

impl GroovyScriptExecutor {
    /// 注册 `language = "groovy"`。
    pub fn register() -> LFResult<()> {
        ScriptExecutorFactory::register("groovy", Self::build)
    }

    fn build(node_id: &str, kind: ScriptKind, script: &str) -> LFResult<Arc<dyn NodeComponent>> {
        let normalized = normalize_script(node_id, script)?;
        build_rhai_component(node_id, kind, &normalized)
    }
}

/// 将 LiteFlow Groovy 常用绑定映射到 Rust 脚本上下文。
///
/// 覆盖 `def`/基础数值类型声明、DefaultContext、`_meta`、ScriptBean 与标准输出。
/// JVM 动态类和任意 import 不属于 Rust 受控执行面，会返回明确加载错误。
/// 对应 Java: `JSR223ScriptExecutor#bindParam`。
fn normalize_script(node_id: &str, script: &str) -> LFResult<String> {
    let mut normalized = Vec::new();
    let mut declared_variables = HashSet::new();
    for raw_line in script.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("import ") || line.starts_with("class ") {
            return Err(script_error(
                node_id,
                "Groovy JVM import/class syntax is outside the Rust controlled runtime",
            ));
        }

        let mut line = line
            .replace("System.out.println(", "print(")
            .replace("_meta.requestData", "input")
            .replace("_meta.cmpData", "cmp_data")
            .replace("_meta.loopObject", "loop_object")
            .replace("_meta.loopIndex", "loop_index")
            .replace("_meta.nodeId", "node_id")
            .replace("_meta.tag", "tag");
        if let Some(expression) = line.strip_prefix("println ") {
            line = format!("print({expression})");
        }
        line = replace_context_calls(node_id, line)?;
        line = replace_script_bean_calls(node_id, line)?;

        if let Some(rest) = declaration_rest(&line) {
            if let Some(variable) = assignment_variable(rest) {
                declared_variables.insert(variable.to_string());
            }
            normalized.push(terminate_statement(format!("let {rest}")));
        } else if let Some(variable) = assignment_variable(&line) {
            if declared_variables.insert(variable.to_string()) {
                normalized.push(terminate_statement(format!("let {line}")));
            } else {
                normalized.push(terminate_statement(line));
            }
        } else {
            normalized.push(terminate_statement(line));
        }
    }
    if normalized.is_empty() {
        return Err(script_error(node_id, "Groovy script cannot be empty"));
    }
    Ok(normalized.join("\n"))
}

fn terminate_statement(statement: String) -> String {
    let trimmed = statement.trim_end();
    if trimmed.ends_with(';') || trimmed.ends_with('{') || trimmed.ends_with('}') {
        statement
    } else {
        format!("{trimmed};")
    }
}

fn declaration_rest(statement: &str) -> Option<&str> {
    [
        "def ", "int ", "long ", "double ", "float ", "boolean ", "String ",
    ]
    .iter()
    .find_map(|prefix| statement.strip_prefix(prefix))
}

fn assignment_variable(statement: &str) -> Option<&str> {
    let (candidate, _) = statement.split_once('=')?;
    let candidate = candidate.trim();
    (!candidate.is_empty()
        && candidate
            .chars()
            .all(|character| character == '_' || character.is_ascii_alphanumeric())
        && candidate
            .chars()
            .next()
            .is_some_and(|character| character == '_' || character.is_ascii_alphabetic()))
    .then_some(candidate)
}

fn replace_context_calls(node_id: &str, mut source: String) -> LFResult<String> {
    for (method, call_kind) in [
        ("defaultContext.getData", ContextCall::Get),
        ("defaultContext.hasData", ContextCall::Has),
        ("defaultContext.setData", ContextCall::Set),
    ] {
        while let Some(start) = source.find(&format!("{method}(")) {
            let open = start + method.len();
            let close = matching_parenthesis(&source, open)
                .ok_or_else(|| script_error(node_id, format!("unclosed {method} call")))?;
            let arguments = split_arguments(&source[open + 1..close]);
            let replacement = match call_kind {
                ContextCall::Get if arguments.len() == 1 => {
                    format!("data[{}]", arguments[0].trim())
                }
                ContextCall::Has if arguments.len() == 1 => {
                    format!("data.contains({})", arguments[0].trim())
                }
                ContextCall::Set if arguments.len() == 2 => {
                    format!("data[{}] = {}", arguments[0].trim(), arguments[1].trim())
                }
                _ => {
                    return Err(script_error(
                        node_id,
                        format!("invalid argument count for {method}"),
                    ));
                }
            };
            source.replace_range(start..=close, &replacement);
        }
    }
    Ok(source)
}

/// 把 Groovy 的 `bean.method(args)` 调用接入受控 ScriptBeanManager。
///
/// Java JSR223 会把 `ScriptBeanManager#getScriptBeanMap` 中的对象逐个放入 Bindings；
/// Rust 无运行期反射，因此把同一调用形态转换为显式 `script_context_call`；
/// 执行级代理优先于全局注册表，方法白名单仍由 `ScriptBeanProxy` 的
/// include/exclude 规则校验。对应 Java:
/// `JSR223ScriptExecutor#bindParam` 与 `ScriptBeanProxy.AopInvocationHandler#invoke`。
fn replace_script_bean_calls(node_id: &str, mut source: String) -> LFResult<String> {
    while let Some(call) = next_script_bean_call(&source) {
        let close = matching_parenthesis(&source, call.open).ok_or_else(|| {
            script_error(
                node_id,
                format!("unclosed script bean call {}.{}", call.bean, call.method),
            )
        })?;
        let arguments = source[call.open + 1..close].trim();
        let replacement = format!(
            "script_context_call(_script_beans, {:?}, {:?}, [{}])",
            call.bean, call.method, arguments
        );
        source.replace_range(call.start..=close, &replacement);
    }
    Ok(source)
}

struct ScriptBeanCall<'a> {
    start: usize,
    open: usize,
    bean: &'a str,
    method: &'a str,
}

fn next_script_bean_call(source: &str) -> Option<ScriptBeanCall<'_>> {
    let bytes = source.as_bytes();
    let mut index = 0;
    let mut quote = None;
    let mut escaped = false;
    while index < bytes.len() {
        let character = bytes[index] as char;
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == active_quote {
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
        if !is_identifier_start(character) {
            index += 1;
            continue;
        }

        let bean_start = index;
        index += 1;
        while index < bytes.len() && is_identifier_continue(bytes[index] as char) {
            index += 1;
        }
        let bean_end = index;
        if bytes.get(index) != Some(&b'.') {
            continue;
        }
        index += 1;
        let method_start = index;
        if index >= bytes.len() || !is_identifier_start(bytes[index] as char) {
            continue;
        }
        index += 1;
        while index < bytes.len() && is_identifier_continue(bytes[index] as char) {
            index += 1;
        }
        let method_end = index;
        while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
            index += 1;
        }
        if bytes.get(index) != Some(&b'(') {
            continue;
        }

        let bean = &source[bean_start..bean_end];
        if matches!(
            bean,
            "data" | "input" | "cmp_data" | "loop_object" | "defaultContext" | "System" | "_meta"
        ) {
            continue;
        }
        return Some(ScriptBeanCall {
            start: bean_start,
            open: index,
            bean,
            method: &source[method_start..method_end],
        });
    }
    None
}

fn is_identifier_start(character: char) -> bool {
    character == '_' || character.is_ascii_alphabetic()
}

fn is_identifier_continue(character: char) -> bool {
    is_identifier_start(character) || character.is_ascii_digit()
}

#[derive(Clone, Copy)]
enum ContextCall {
    Get,
    Has,
    Set,
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

fn script_error(node_id: &str, message: impl Into<String>) -> liteflow_core::LiteflowError {
    liteflow_core::LiteflowError::Script {
        node: node_id.to_string(),
        msg: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_script;

    #[test]
    fn normalizes_default_context_and_meta_bindings() {
        let normalized = normalize_script(
            "groovy",
            r#"
                def a = 3
                int b = 2
                defaultContext.setData("s1", a * b)
                def score = defaultContext.getData("s1")
                def present = defaultContext.hasData("s1")
                def request = _meta.requestData
                println request
            "#,
        )
        .unwrap();
        assert!(normalized.contains("let a = 3"));
        assert!(normalized.contains("let b = 2"));
        assert!(normalized.contains("data[\"s1\"] = a * b"));
        assert!(normalized.contains("let score = data[\"s1\"]"));
        assert!(normalized.contains("let present = data.contains(\"s1\")"));
        assert!(normalized.contains("let request = input"));
        assert!(normalized.contains("print(request);"));
    }

    #[test]
    fn normalizes_script_bean_calls_through_controlled_bridge() {
        let normalized = normalize_script(
            "groovy",
            r#"defaultContext.setData("demo", demoBean.sayHello("kobe"))"#,
        )
        .unwrap();
        assert_eq!(
            normalized,
            r#"data["demo"] = script_context_call(_script_beans, "demoBean", "sayHello", ["kobe"]);"#
        );
    }

    #[test]
    fn rejects_jvm_class_syntax_explicitly() {
        assert!(normalize_script("groovy", "class Student {}").is_err());
    }
}
