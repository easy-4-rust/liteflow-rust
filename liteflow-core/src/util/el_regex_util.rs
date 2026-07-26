//! 对应 Java 类：com.yomahub.liteflow.util.ElRegexUtil（v2.16.0 新增，v2.10.0 基线中不存在）
//!
//! 抽象链占位符工具：识别 {{name}} 形式的抽象节点占位符（链继承核心语义）。

use crate::exception::{LFResult, LiteflowError};

/// REGEX_ABSTRACT_HOLDER 语义：{{name}} 且后不接 =
fn find_placeholders(el: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = el.chars().collect();
    let mut i = 0;
    while i + 1 < chars.len() {
        if chars[i] == '{' && chars[i + 1] == '{' {
            let mut j = i + 2;
            while j < chars.len() && (chars[j].is_whitespace()) {
                j += 1;
            }
            let start = j;
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            let name: String = chars[start..j].iter().collect();
            let mut k = j;
            while k < chars.len() && chars[k].is_whitespace() {
                k += 1;
            }
            if !name.is_empty() && k + 1 < chars.len() && chars[k] == '}' && chars[k + 1] == '}' {
                // 后接 = 的是实现赋值，不是占位符
                let mut m = k + 2;
                while m < chars.len() && chars[m].is_whitespace() {
                    m += 1;
                }
                if m >= chars.len() || chars[m] != '=' {
                    out.push(name);
                }
                i = k + 2;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// isAbstractChain
pub fn is_abstract_chain(el: &str) -> bool {
    !find_placeholders(el).is_empty()
}

/// replaceAbstractChain：把父链 EL 中的 {{holder}} 替换为子链 EL 中的
/// {{holder}} = 实现; 赋值。缺少实现时报 ParseException（语义对齐 Java）。
pub fn replace_abstract_chain(abstract_el: &str, impl_el: &str) -> LFResult<String> {
    let mut result = abstract_el.to_string();
    for holder in find_placeholders(abstract_el) {
        // 在 impl 中找 "{{holder}} = ...;" 赋值
        let assign = find_assignment(impl_el, &holder).ok_or_else(|| {
            LiteflowError::Parse(format!(
                "missing implementation of {{{{{holder}}}}} in expression \r\n{impl_el}"
            ))
        })?;
        // 替换所有出现的 {{holder}}（含空白变体）
        result = replace_placeholder(&result, &holder, &assign);
    }
    Ok(result)
}

/// 提取 "{{holder}} = xxx;" 中的 xxx
fn find_assignment(el: &str, holder: &str) -> Option<String> {
    // 定位 "{{holder}}" 且后接 '='
    let mut search_from = 0;
    loop {
        let start = el[search_from..].find("{{")? + search_from;
        let end = el[start..].find("}}")? + start;
        let name = el[start + 2..end].trim();
        let after = &el[end + 2..];
        let after_trim = after.trim_start();
        if name == holder && after_trim.starts_with('=') {
            let expr = &after_trim[1..];
            let semi = expr.find(';')?;
            return Some(expr[..semi].trim().to_string());
        }
        search_from = end + 2;
        if search_from >= el.len() {
            return None;
        }
    }
}

/// 替换 {{holder}}（容忍内部空白）为 replacement
fn replace_placeholder(el: &str, holder: &str, replacement: &str) -> String {
    let mut out = String::new();
    let mut rest = el;
    while let Some(start) = rest.find("{{") {
        if let Some(rel_end) = rest[start..].find("}}") {
            let end = start + rel_end;
            let name = rest[start + 2..end].trim();
            // 只替换纯占位（后不接 =）
            let after = rest[end + 2..].trim_start();
            if name == holder && !after.starts_with('=') {
                out.push_str(&rest[..start]);
                out.push_str(replacement);
                rest = &rest[end + 2..];
                continue;
            }
        }
        // 不是目标，原样保留到 {{
        out.push_str(&rest[..start + 2]);
        rest = &rest[start + 2..];
    }
    out.push_str(rest);
    out
}

/// 对应 ElRegexUtil.normalize（2.16，execute2RespWithEL 用）：
/// 剔除 EL 中多余空格，将单引号变为双引号，并在末尾保留一个分号
pub fn normalize_el(el_str: &str) -> String {
    let s: String = el_str
        .replace('\'', "\"")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let trimmed = s.trim_end_matches(';');
    format!("{trimmed};")
}

#[cfg(test)]
mod tests {
    use super::replace_abstract_chain;

    #[test]
    fn abstract_chain_replace() {
        let parent = "THEN(a, {{x}}, WHEN({{y}}, d))";
        let child = "{{x}} = b; {{y}} = IF(c1, c2);";
        let r = replace_abstract_chain(parent, child).unwrap();
        assert_eq!(r, "THEN(a, b, WHEN(IF(c1, c2), d))");
    }

    #[test]
    fn missing_impl_error() {
        let parent = "THEN(a, {{x}})";
        let child = "THEN(b)";
        assert!(replace_abstract_chain(parent, child).is_err());
    }
}
