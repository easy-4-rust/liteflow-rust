//! 对应 Java: com.yomahub.liteflow.util.LiteflowContextRegexMatcher

use serde_json::Value;

/// 在具名 JSON 上下文中搜索属性或执行 setter 语义。
///
/// Java 依赖 QLExpress 反射任意 Bean；Rust 以 serde JSON 对象作为安全边界，
/// 支持 `address.city`、`contextAlias.address.city` 与 `setName`。
pub struct LiteflowContextRegexMatcher;

impl LiteflowContextRegexMatcher {
    /// 按表达式搜索第一个非空上下文值。
    #[must_use]
    pub fn search_context(context_list: &[(String, Value)], reg_pattern: &str) -> Option<Value> {
        let segments = path_segments(reg_pattern);
        for (_, context) in context_list {
            if let Some(value) = lookup(context, &segments) {
                return Some(value.clone());
            }
        }
        let (alias, remaining) = segments.split_first()?;
        context_list
            .iter()
            .find(|(name, _)| name == alias)
            .and_then(|(_, context)| lookup(context, remaining))
            .cloned()
    }

    /// 在首个匹配上下文上执行 setter。
    ///
    /// `setName` 映射为字段 `name`；点路径最后一段同样作为待写字段。
    /// 返回是否成功写入，对应 Java 内部的 `flag`。
    pub fn search_and_set_context(
        context_list: &mut [(String, Value)],
        method_expression: &str,
        arguments: &[Value],
    ) -> bool {
        let Some(value) = arguments.first().cloned() else {
            return false;
        };
        let segments = setter_segments(method_expression);
        for (_, context) in context_list.iter_mut() {
            if set_path(context, &segments, value.clone()) {
                return true;
            }
        }
        let Some((alias, remaining)) = segments.split_first() else {
            return false;
        };
        context_list
            .iter_mut()
            .find(|(name, _)| name == alias)
            .is_some_and(|(_, context)| set_path(context, remaining, value))
    }
}

fn path_segments(path: &str) -> Vec<&str> {
    path.trim()
        .split('.')
        .filter(|part| !part.is_empty())
        .collect()
}

fn setter_segments(method_expression: &str) -> Vec<String> {
    let mut segments = method_expression
        .trim_end_matches("()")
        .split('.')
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if let Some(last) = segments.last_mut()
        && let Some(property) = last.strip_prefix("set")
        && !property.is_empty()
    {
        let mut chars = property.chars();
        let first = chars.next().unwrap().to_lowercase().collect::<String>();
        *last = format!("{first}{}", chars.as_str());
    }
    segments
}

fn lookup<'a>(value: &'a Value, segments: &[&str]) -> Option<&'a Value> {
    segments
        .iter()
        .try_fold(value, |current, segment| current.get(*segment))
}

fn set_path(value: &mut Value, segments: &[String], replacement: Value) -> bool {
    let Some((last, parents)) = segments.split_last() else {
        return false;
    };
    let mut current = value;
    for segment in parents {
        let Some(next) = current.get_mut(segment) else {
            return false;
        };
        current = next;
    }
    let Some(object) = current.as_object_mut() else {
        return false;
    };
    if !object.contains_key(last) {
        return false;
    }
    object.insert(last.clone(), replacement);
    true
}
