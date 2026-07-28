//! 对应 Java: com.yomahub.liteflow.util.LiteflowContextRegexMatcher

use std::collections::HashMap;

use qlexpress::DataValue;
use qlexpress::runtime::data::IndexMap;
use serde_json::{Map, Number, Value};

use super::QlExpressUtils;

/// 在具名 JSON 上下文中搜索属性或执行 setter 语义。
///
/// Java 依赖 QLExpress 反射任意 Bean；Rust 将 serde JSON 转为 QLExpress Map，
/// 由真实 QVM 执行属性读取和赋值，同时以 JSON 对象限制宿主反射边界。支持
/// `address.city`、`contextAlias.address.city` 与 `setName`。
pub struct LiteflowContextRegexMatcher;

impl LiteflowContextRegexMatcher {
    /// 按表达式搜索第一个非空上下文值。
    ///
    /// 参数 `context_list` 是具名 JSON 上下文，`reg_pattern` 是 Java 同名参数；
    /// 返回首个 QLExpress 求值成功且非空的 JSON 值。对应 Java:
    /// `LiteflowContextRegexMatcher#searchContext`。
    #[must_use]
    pub fn search_context(context_list: &[(String, Value)], reg_pattern: &str) -> Option<Value> {
        let runner = QlExpressUtils::get_context_search_express_runner();

        // 与 Java 一致，先把表达式分别应用到每个具名上下文，首个非 null
        // 结果获胜。单个上下文失败不会阻断后续候选。
        for (alias, context_value) in context_list {
            let context = HashMap::from([(alias.clone(), json_to_data_value(context_value))]);
            if let Ok(result) = runner.execute(&format!("{alias}.{reg_pattern}"), context)
                && !result.is_null()
                && let Some(result) = data_value_to_json(&result)
            {
                return Some(result);
            }
        }

        // 首轮未命中时按 Java 的 contextMap.<表达式> 规则再次求值，从而支持
        // 调用方在表达式中显式写出上下文别名。
        let context_map = DataValue::map(IndexMap::from_entries(
            context_list
                .iter()
                .map(|(alias, value)| (DataValue::Str(alias.clone()), json_to_data_value(value)))
                .collect(),
        ));
        runner
            .execute(
                &format!("contextMap.{reg_pattern}"),
                HashMap::from([("contextMap".to_string(), context_map)]),
            )
            .ok()
            .filter(|result| !result.is_null())
            .and_then(|result| data_value_to_json(&result))
    }

    /// 在首个匹配上下文上执行 setter。
    ///
    /// `setName` 映射为字段 `name`；点路径最后一段同样作为待写字段。
    /// 参数 `context_list`、`method_expression`、`arguments` 分别对应 Java 的
    /// contextList、methodExpress、args；返回是否成功写入，对应 Java 内部的
    /// `flag`。对应 Java: `LiteflowContextRegexMatcher#searchAndSetContext`。
    pub fn search_and_set_context(
        context_list: &mut [(String, Value)],
        method_expression: &str,
        arguments: &[Value],
    ) -> bool {
        let Some(replacement) = arguments.first() else {
            return false;
        };
        let segments = setter_segments(method_expression);
        let Some((context_index, path)) = resolve_setter_target(context_list, &segments) else {
            return false;
        };
        let (alias, context_value) = &mut context_list[context_index];

        let qlexpress_value = json_to_data_value(context_value);
        let qlexpress_context = HashMap::from([
            (alias.to_string(), qlexpress_value.clone()),
            ("arg0".to_string(), json_to_data_value(replacement)),
        ]);
        let expression = format!("{alias}.{} = arg0", path.join("."));
        if QlExpressUtils::get_context_search_express_runner()
            .execute(&expression, qlexpress_context)
            .is_ok()
            && let Some(updated) = data_value_to_json(&qlexpress_value)
        {
            *context_value = updated;
            return true;
        }
        false
    }
}

fn resolve_setter_target(
    context_list: &[(String, Value)],
    segments: &[String],
) -> Option<(usize, Vec<String>)> {
    let (first, remaining) = segments.split_first()?;
    if let Some((index, _)) = context_list
        .iter()
        .enumerate()
        .find(|(_, (name, _))| name == first)
        && !remaining.is_empty()
    {
        return Some((index, remaining.to_vec()));
    }
    context_list
        .iter()
        .enumerate()
        .find(|(_, (_, context))| path_exists(context, segments))
        .map(|(index, _)| (index, segments.to_vec()))
}

fn path_exists(value: &Value, segments: &[String]) -> bool {
    segments
        .iter()
        .try_fold(value, |current, segment| current.get(segment))
        .is_some()
}

fn json_to_data_value(value: &Value) -> DataValue {
    match value {
        Value::Null => DataValue::Null,
        Value::Bool(value) => DataValue::Bool(*value),
        Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                i32::try_from(value)
                    .map(DataValue::Int)
                    .unwrap_or(DataValue::Long(value))
            } else {
                DataValue::Double(value.as_f64().unwrap_or_default())
            }
        }
        Value::String(value) => DataValue::Str(value.clone()),
        Value::Array(values) => DataValue::list(values.iter().map(json_to_data_value).collect()),
        Value::Object(values) => DataValue::map(IndexMap::from_entries(
            values
                .iter()
                .map(|(key, value)| (DataValue::Str(key.clone()), json_to_data_value(value)))
                .collect(),
        )),
    }
}

fn data_value_to_json(value: &DataValue) -> Option<Value> {
    match value {
        DataValue::Null => Some(Value::Null),
        DataValue::Bool(value) => Some(Value::Bool(*value)),
        DataValue::Byte(value) => Some(Value::Number(Number::from(*value))),
        DataValue::Short(value) => Some(Value::Number(Number::from(*value))),
        DataValue::Int(value) => Some(Value::Number(Number::from(*value))),
        DataValue::Long(value) => Some(Value::Number(Number::from(*value))),
        DataValue::Float(value) => Number::from_f64(f64::from(*value)).map(Value::Number),
        DataValue::Double(value) => Number::from_f64(*value).map(Value::Number),
        DataValue::BigInt(value) => serde_json::from_str(&value.to_string()).ok(),
        DataValue::BigDec(value) => serde_json::from_str(value).ok(),
        DataValue::Char(value) => Some(Value::String(value.to_string())),
        DataValue::Str(value) => Some(Value::String(value.clone())),
        DataValue::List(values) | DataValue::Array(values) => values
            .borrow()
            .iter()
            .map(data_value_to_json)
            .collect::<Option<Vec<_>>>()
            .map(Value::Array),
        DataValue::Map(values) => {
            let mut object = Map::new();
            for (key, value) in values.borrow().entries() {
                let DataValue::Str(key) = key else {
                    return None;
                };
                object.insert(key.clone(), data_value_to_json(value)?);
            }
            Some(Value::Object(object))
        }
        DataValue::Lambda(_) | DataValue::Object(_) => None,
    }
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::LiteflowContextRegexMatcher;

    #[test]
    fn search_context_uses_real_qlexpress_property_access() {
        let context_list = vec![
            ("first".to_string(), json!({"address": null})),
            (
                "second".to_string(),
                json!({"address": {"city": "Hangzhou"}}),
            ),
        ];
        assert_eq!(
            LiteflowContextRegexMatcher::search_context(&context_list, "address.city"),
            Some(json!("Hangzhou"))
        );
        assert_eq!(
            LiteflowContextRegexMatcher::search_context(&context_list, "second.address.city"),
            Some(json!("Hangzhou"))
        );
    }

    #[test]
    fn search_and_set_context_writes_back_qlexpress_map_assignment() {
        let mut context_list = vec![("user".to_string(), json!({"profile": {"name": "before"}}))];
        assert!(LiteflowContextRegexMatcher::search_and_set_context(
            &mut context_list,
            "profile.setName",
            &[json!("after")],
        ));
        assert_eq!(context_list[0].1["profile"]["name"], json!("after"));
    }
}
