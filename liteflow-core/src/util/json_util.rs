//! JSON 序列化与反序列化工具。
//!
//! 对应 Java: `com.yomahub.liteflow.util.JsonUtil`。

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::exception::JsonProcessException;

/// 基于 serde/serde_json 提供 Java `JsonUtil` 的空值和异常语义。
pub struct JsonUtil;

impl JsonUtil {
    /// 将对象序列化为 JSON 字符串。
    ///
    /// 参数为 `None` 时返回 `Ok(None)`，对应 Java 传入 `null` 时返回 `null`。
    pub fn to_json_string<T: Serialize>(
        object: Option<&T>,
    ) -> Result<Option<String>, JsonProcessException> {
        let Some(object) = object else {
            return Ok(None);
        };
        serde_json::to_string(object).map(Some).map_err(|error| {
            JsonProcessException::new(format!(
                "Error while writing value as string[{}],reason: {error}",
                std::any::type_name::<T>()
            ))
        })
    }

    /// 将文本解析为通用 JSON 值。
    ///
    /// 空字符串返回 `Ok(None)`；非法 JSON 返回 `JsonProcessException`。
    pub fn parse_value(text: &str) -> Result<Option<Value>, JsonProcessException> {
        if text.is_empty() {
            return Ok(None);
        }
        serde_json::from_str(text).map(Some).map_err(|error| {
            JsonProcessException::new(format!("Error while parsing text [{text}],reason: {error}"))
        })
    }

    /// 将 JSON 文本解析为指定对象。
    ///
    /// 空字符串返回 `Ok(None)`。对应 Java:
    /// `JsonUtil#parseObject(String, Class<T>)`。
    pub fn parse_object<T: DeserializeOwned>(
        json: &str,
    ) -> Result<Option<T>, JsonProcessException> {
        if json.is_empty() {
            return Ok(None);
        }
        serde_json::from_str(json).map(Some).map_err(|error| {
            JsonProcessException::new(format!("Error while parsing text [{json}],reason: {error}"))
        })
    }

    /// 将 JSON 数组解析为对象列表。
    ///
    /// 空字符串返回空列表。对应 Java: `JsonUtil#parseList`。
    pub fn parse_list<T: DeserializeOwned>(json: &str) -> Result<Vec<T>, JsonProcessException> {
        if json.is_empty() {
            return Ok(Vec::new());
        }
        serde_json::from_str(json).map_err(|error| {
            JsonProcessException::new(format!("Error while parsing text [{json}],reason: {error}"))
        })
    }
}
