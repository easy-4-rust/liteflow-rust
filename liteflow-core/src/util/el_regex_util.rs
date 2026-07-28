//! 对应 Java 类：com.yomahub.liteflow.util.ElRegexUtil（v2.16.0 新增，v2.10.0 基线中不存在）
//!
//! 抽象链占位符工具：识别 {{name}} 形式的抽象节点占位符（链继承核心语义）。

use crate::exception::{LFResult, LiteflowError};

/// LiteFlow EL 正则与抽象链处理工具。
///
/// 负责识别、替换抽象链占位符，并规范化动态执行的 EL 文本。所有公开入口均保留
/// Java 工具类的静态方法语义。
///
/// 对应 Java: `com.yomahub.liteflow.util.ElRegexUtil`。
pub struct ElRegexUtil;

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

impl ElRegexUtil {
    /// 判断 EL 是否包含尚未实现的抽象链占位符。
    ///
    /// 参数 `el` 为待检查的链路表达式；包含 `{{name}}` 且其后不是赋值符号时返回
    /// `true`。对应 Java: `ElRegexUtil#isAbstractChain`。
    #[must_use]
    pub fn is_abstract_chain(el: &str) -> bool {
        !find_placeholders(el).is_empty()
    }

    /// 使用子链实现替换父链中的抽象占位符。
    ///
    /// 参数 `abstract_el` 为父链 EL，`impl_el` 为包含 `{{holder}} = expression;`
    /// 赋值的子链 EL；缺少任一实现时返回解析错误。
    /// 对应 Java: `ElRegexUtil#replaceAbstractChain`。
    pub fn replace_abstract_chain(abstract_el: &str, impl_el: &str) -> LFResult<String> {
        let mut result = abstract_el.to_string();
        for holder in find_placeholders(abstract_el) {
            // 逐个解析实现赋值，避免未实现占位符进入后续 EL 解析器。
            let assign = find_assignment(impl_el, &holder).ok_or_else(|| {
                LiteflowError::Parse(format!(
                    "missing implementation of {{{{{holder}}}}} in expression \r\n{impl_el}"
                ))
            })?;
            // 替换所有出现的同名占位符，并兼容花括号内的空白。
            result = replace_placeholder(&result, &holder, &assign);
        }
        Ok(result)
    }

    /// 规范化用于动态执行的 EL 文本。
    ///
    /// 参数 `el_str` 为原始 EL；返回值会把单引号替换为双引号、删除全部空白，并
    /// 把任意数量的尾部分号收敛为一个分号。
    /// 对应 Java: `ElRegexUtil#normalize`。
    #[must_use]
    pub fn normalize(el_str: &str) -> String {
        let normalized: String = el_str
            .replace('\'', "\"")
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect();
        let trimmed = normalized.trim_end_matches(';');
        format!("{trimmed};")
    }
}

#[cfg(test)]
mod tests {
    use super::ElRegexUtil;

    #[test]
    fn abstract_chain_replace() {
        let parent = "THEN(a, {{x}}, WHEN({{y}}, d))";
        let child = "{{x}} = b; {{y}} = IF(c1, c2);";
        let r = ElRegexUtil::replace_abstract_chain(parent, child).unwrap();
        assert_eq!(r, "THEN(a, b, WHEN(IF(c1, c2), d))");
    }

    #[test]
    fn missing_impl_error() {
        let parent = "THEN(a, {{x}})";
        let child = "THEN(b)";
        assert!(ElRegexUtil::replace_abstract_chain(parent, child).is_err());
    }

    #[test]
    fn normalize_matches_java_replacement_order() {
        assert_eq!(
            ElRegexUtil::normalize(" THEN( a.tag('A B') ) ;;; \n"),
            "THEN(a.tag(\"AB\"));"
        );
    }
}
