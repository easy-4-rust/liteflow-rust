//! 对应 Java: `com.yomahub.liteflow.script.kotlin.KotlinScriptExecutor`。

use std::collections::HashSet;
use std::sync::Arc;

use liteflow_core::LFResult;
use liteflow_core::core::NodeComponent;
use liteflow_core::script::{ScriptExecutorFactory, ScriptKind, build_rhai_component};

/// Kotlin LiteFlow 常用绑定语义的 Rust 受控执行器。
///
/// Java 版本通过 JSR223 Kotlin 引擎编译脚本；Rust 版本保留 `val`/`var`、显式类型、
/// `String.toInt()`、DefaultContext 与 `_meta` 的可验证语义，并编译为受控 Rhai AST。
/// JVM classpath、任意 import、类/对象/函数声明不属于当前 Rust 执行面，会明确报错。
pub struct KotlinScriptExecutor;

impl KotlinScriptExecutor {
    /// 注册 `language = "kotlin"` 的组件构建器。
    ///
    /// 对应 Java: `KotlinScriptExecutor#scriptType` 与 ServiceLoader 注册。
    pub fn register() -> LFResult<()> {
        ScriptExecutorFactory::register("kotlin", Self::build)
    }

    fn build(node_id: &str, kind: ScriptKind, script: &str) -> LFResult<Arc<dyn NodeComponent>> {
        let normalized = normalize_script(node_id, script)?;
        build_rhai_component(node_id, kind, &normalized)
    }
}

/// 将 Kotlin 基线语法转换为 Rust 受控脚本。
///
/// `val` 在编译期禁止再次赋值，`var` 保留可变语义；显式基础类型会校验明显的字面量
/// 不匹配。对应 Java: `JSR223ScriptExecutor#compile` 和 Kotlin 编译器类型检查。
fn normalize_script(node_id: &str, script: &str) -> LFResult<String> {
    let mut immutable_variables = HashSet::new();
    let mut mutable_variables = HashSet::new();
    let mut normalized = Vec::new();

    for raw_line in script.lines() {
        let source = raw_line.trim();
        if source.is_empty() {
            continue;
        }
        if ["import ", "package ", "class ", "object ", "fun "]
            .iter()
            .any(|prefix| source.starts_with(prefix))
        {
            return Err(script_error(
                node_id,
                "Kotlin JVM declaration/import syntax is outside the Rust controlled runtime",
            ));
        }

        let mut line = source
            .replace("println(", "print(")
            .replace("_meta.requestData", "input")
            .replace("_meta.cmpData", "cmp_data")
            .replace("_meta.loopObject", "loop_object")
            .replace("_meta.loopIndex", "loop_index")
            .replace("_meta.nodeId", "node_id")
            .replace("_meta.tag", "tag");
        line = replace_context_calls(node_id, line)?;

        if let Some(declaration) = parse_declaration(&line) {
            validate_declared_type(node_id, declaration.declared_type, declaration.expression)?;
            let expression = normalize_conversion(declaration.expression);
            if declaration.mutable {
                mutable_variables.insert(declaration.name.to_string());
            } else {
                immutable_variables.insert(declaration.name.to_string());
            }
            normalized.push(terminate_statement(format!(
                "let {} = {expression}",
                declaration.name
            )));
            continue;
        }

        if let Some(variable) = assignment_variable(&line) {
            if immutable_variables.contains(variable) {
                return Err(script_error(
                    node_id,
                    format!("Kotlin val [{variable}] cannot be reassigned"),
                ));
            }
            if !mutable_variables.contains(variable) {
                return Err(script_error(
                    node_id,
                    format!("Kotlin variable [{variable}] must be declared with val or var"),
                ));
            }
        }
        normalized.push(terminate_statement(line));
    }

    if normalized.is_empty() {
        return Err(script_error(node_id, "Kotlin script cannot be empty"));
    }
    Ok(normalized.join("\n"))
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
    use super::normalize_script;

    #[test]
    fn validates_kotlin_typed_declarations_and_conversion() {
        let normalized = normalize_script(
            "kotlin",
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
        assert!(normalize_script("kotlin", r#"val number: Int = "123""#).is_err());
        assert!(normalize_script("kotlin", "val number: Int = 1\nnumber = 2").is_err());
    }

    #[test]
    fn normalizes_default_context_and_meta() {
        let normalized = normalize_script(
            "kotlin",
            r#"
                val score: Int = input.score
                defaultContext.setData("score", score)
                val node: String = _meta.nodeId
            "#,
        )
        .unwrap();
        assert!(normalized.contains("let score = input.score;"));
        assert!(normalized.contains(r#"data["score"] = score;"#));
        assert!(normalized.contains("let node = node_id;"));
    }
}
