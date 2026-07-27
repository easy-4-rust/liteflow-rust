//! 对应 Java: `com.yomahub.liteflow.script.aviator.AviatorScriptExecutor`。

use std::sync::Arc;
use std::collections::HashSet;

use liteflow_core::LFResult;
use liteflow_core::core::NodeComponent;
use liteflow_core::script::{ScriptExecutorFactory, ScriptKind, build_rhai_component};

/// Aviator 公共表达式子集执行器。
pub struct AviatorScriptExecutor;

impl AviatorScriptExecutor {
    /// 注册 `language = "aviator"`。
    pub fn register() -> LFResult<()> {
        ScriptExecutorFactory::register("aviator", Self::build)
    }

    fn build(node_id: &str, kind: ScriptKind, script: &str) -> LFResult<Arc<dyn NodeComponent>> {
        let normalized = normalize_script(node_id, script)?;
        build_rhai_component(node_id, kind, &normalized)
    }
}

/// 将 LiteFlow v2.16.0 Aviator 基线语法映射到受控 Rust 脚本原语。
///
/// `use` 导入仅允许基线中的 Date/DateUtil；时间表达式在执行期调用
/// `aviator_now()`；`setData(defaultContext, key, value)` 写入共享 `data`。
/// 对应 Java: `AviatorScriptExecutor` 通过 JSR223 绑定 `defaultContext`。
fn normalize_script(node_id: &str, script: &str) -> LFResult<String> {
    let mut normalized = Vec::new();
    let mut declared_variables = HashSet::new();
    for raw_line in script.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(import_name) = line
            .strip_prefix("use ")
            .and_then(|value| value.strip_suffix(';'))
        {
            if matches!(
                import_name.trim(),
                "java.util.Date" | "cn.hutool.core.date.DateUtil"
            ) {
                continue;
            }
            return Err(script_error(
                node_id,
                format!("unsupported Aviator import [{import_name}]"),
            ));
        }

        let line = line
            .replace("DateUtil.formatDateTime(new Date())", "aviator_now()")
            .replace("println(", "print(");
        if line.starts_with("setData(") {
            normalized.push(normalize_set_data(node_id, &line)?);
        } else if let Some(variable) = line
            .strip_prefix("let ")
            .and_then(assignment_variable)
        {
            declared_variables.insert(variable.to_string());
            normalized.push(line);
        } else if let Some(variable) = assignment_variable(&line) {
            if declared_variables.insert(variable.to_string()) {
                normalized.push(format!("let {line}"));
            } else {
                normalized.push(line);
            }
        } else {
            normalized.push(line);
        }
    }
    if normalized.is_empty() {
        return Err(script_error(node_id, "Aviator script cannot be empty"));
    }
    Ok(normalized.join("\n"))
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

fn normalize_set_data(node_id: &str, line: &str) -> LFResult<String> {
    let arguments = line
        .strip_prefix("setData(")
        .and_then(|value| value.strip_suffix(';'))
        .and_then(|value| value.strip_suffix(')'))
        .ok_or_else(|| script_error(node_id, "invalid setData call"))?;
    let arguments = split_arguments(arguments);
    if arguments.len() != 3 || arguments[0].trim() != "defaultContext" {
        return Err(script_error(
            node_id,
            "setData requires (defaultContext, key, value)",
        ));
    }
    let key = arguments[1].trim();
    if !(key.starts_with('"') && key.ends_with('"')) {
        return Err(script_error(
            node_id,
            "setData key must be a string literal",
        ));
    }
    Ok(format!("data[{key}] = {};", arguments[2].trim()))
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
    fn normalizes_java_aviator_baseline_syntax() {
        let script = r#"
            use java.util.Date;
            use cn.hutool.core.date.DateUtil;
            let d = DateUtil.formatDateTime(new Date());
            println(d);
            a = 2;
            b = 3;
            setData(defaultContext, "s1", a*b);
        "#;
        let normalized = normalize_script("s1", script).unwrap();
        assert!(normalized.contains("let d = aviator_now();"));
        assert!(normalized.contains("print(d);"));
        assert!(normalized.contains("let a = 2;"));
        assert!(normalized.contains("let b = 3;"));
        assert!(normalized.contains("data[\"s1\"] = a*b;"));
    }

    #[test]
    fn rejects_unmapped_java_imports() {
        assert!(normalize_script("s1", "use java.io.File;").is_err());
    }
}
