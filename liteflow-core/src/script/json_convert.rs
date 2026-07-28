//! serde_json::Value 与 rhai::Dynamic 的双向转换。

use rhai::{Dynamic, Map};
use serde_json::Value;

/// 将 serde JSON 值转换为 Rhai 动态值。
///
/// 参数 `v` 可以是 null、布尔、数字、字符串、数组或对象；返回值递归保持相同
/// 数据结构。该 Rust 专用入口承接 Java 脚本引擎的参数对象转换。
#[must_use]
pub fn json_to_dynamic(v: &Value) -> Dynamic {
    match v {
        Value::Null => Dynamic::UNIT,
        Value::Bool(b) => Dynamic::from(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                Dynamic::from(f)
            } else {
                Dynamic::UNIT
            }
        }
        Value::String(s) => Dynamic::from(s.clone()),
        Value::Array(a) => {
            let arr: rhai::Array = a.iter().map(json_to_dynamic).collect();
            Dynamic::from(arr)
        }
        Value::Object(o) => {
            let mut m = Map::new();
            for (k, v) in o {
                m.insert(k.clone().into(), json_to_dynamic(v));
            }
            Dynamic::from(m)
        }
    }
}

/// 将 Rhai 动态值转换为 serde JSON 值。
///
/// 参数 `d` 是脚本执行结果；标准标量、数组和 Map 递归转换，其他 Rhai 类型以
/// 字符串表示。该 Rust 专用入口承接 Java 脚本执行器的返回值转换。
#[must_use]
pub fn dynamic_to_json(d: &Dynamic) -> Value {
    if d.is_unit() {
        Value::Null
    } else if let Ok(b) = d.as_bool() {
        Value::Bool(b)
    } else if let Ok(i) = d.as_int() {
        Value::from(i)
    } else if let Ok(f) = d.as_float() {
        Value::from(f)
    } else if d.is_string() {
        Value::String(d.to_string())
    } else if d.is_array() {
        let arr = d.clone().into_array().unwrap_or_default();
        Value::Array(arr.iter().map(dynamic_to_json).collect())
    } else if d.is_map() {
        let m = d.read_lock::<Map>().map(|m| m.clone()).unwrap_or_default();
        let mut obj = serde_json::Map::new();
        for (k, v) in m {
            obj.insert(k.to_string(), dynamic_to_json(&v));
        }
        Value::Object(obj)
    } else {
        Value::String(d.to_string())
    }
}
