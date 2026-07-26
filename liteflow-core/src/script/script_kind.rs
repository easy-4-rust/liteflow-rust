//! Rust 端脚本组件类别。

use serde_json::Value;

use crate::exception::{LFResult, LiteflowError};

/// 对应 `NodeTypeEnum` 中的脚本节点类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptKind {
    /// 普通脚本。
    Common,
    /// 布尔脚本。
    Boolean,
    /// 选择脚本。
    Switch,
    /// 循环次数脚本。
    For,
    /// 迭代集合脚本。
    Iterator,
}

impl ScriptKind {
    /// 根据 Java `NodeTypeEnum` code 解析脚本类别。
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "script" => Some(Self::Common),
            "boolean_script" => Some(Self::Boolean),
            "switch_script" => Some(Self::Switch),
            "for_script" => Some(Self::For),
            "iterator_script" => Some(Self::Iterator),
            _ => None,
        }
    }

    /// 校验脚本返回值类型。
    ///
    /// 对应 Java `ScriptBooleanComponent`、`ScriptSwitchComponent` 和
    /// `ScriptForComponent` 对 `processScript` 返回值的强制类型约束。
    pub fn check_return(self, node_id: &str, value: Value) -> LFResult<Value> {
        let valid = match self {
            Self::Common => true,
            Self::Boolean => value.is_boolean(),
            Self::Switch => value.is_string() || value.is_null(),
            Self::For => value.is_number(),
            Self::Iterator => value.is_array(),
        };
        if valid {
            return Ok(value);
        }
        let expect = match self {
            Self::Boolean => "boolean",
            Self::Switch => "string",
            Self::For => "number",
            Self::Iterator => "array",
            Self::Common => unreachable!(),
        };
        Err(LiteflowError::NodeTypeError {
            node: node_id.to_string(),
            expect: expect.to_string(),
            actual: value.to_string(),
        })
    }
}
