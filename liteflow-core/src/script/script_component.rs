//! 对应 core 包的脚本组件族：
//! ScriptCommonComponent / ScriptBooleanComponent / ScriptSwitchComponent /
//! ScriptForComponent（Rust 版合并为一个对象，kind 区分类型语义）。

use super::script_executor::RhaiScriptExecutor;
use crate::core::node_component::NodeComponent;
use crate::exception::{LFResult, LiteflowError};
use crate::slot::CmpContext;
use async_trait::async_trait;
use rhai::AST;
use serde_json::Value;

/// 对应 NodeTypeEnum 中的脚本类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptKind {
    /// script —— 普通脚本
    Common,
    /// boolean_script —— 布尔脚本（IF/WHILE/BREAK）
    Boolean,
    /// switch_script —— 选择脚本
    Switch,
    /// for_script —— 循环次数脚本
    For,
    /// iterator 脚本（Java 由 script 节点返回集合充当，Rust 显式标注）
    Iterator,
}

impl ScriptKind {
    /// 对应 NodeTypeEnum.getEnumByCode
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
}

/// 脚本组件：构建期编译（对应 isCompiled），执行期求值并校验返回类型
pub struct ScriptComponent {
    node_id: String,
    kind: ScriptKind,
    executor: RhaiScriptExecutor,
    ast: AST,
    /// 非编译型语言的脚本原文（如 lua）
    #[allow(dead_code)] // 默认特性下仅 lua 分支读取
    raw_script: Option<String>,
}

impl ScriptComponent {
    pub fn new(node_id: &str, kind: ScriptKind, script: &str) -> LFResult<Self> {
        let executor = RhaiScriptExecutor::new();
        let ast = executor.compile(node_id, script)?;
        Ok(Self {
            node_id: node_id.to_string(),
            kind,
            executor,
            ast,
            raw_script: None,
        })
    }

    /// Lua 脚本组件（feature "lua"）
    #[cfg(feature = "lua")]
    pub fn new_lua(node_id: &str, kind: ScriptKind, script: &str) -> LFResult<Self> {
        let executor = RhaiScriptExecutor::new();
        let ast = executor.compile(node_id, "()")?; // 占位，不执行
        Ok(Self {
            node_id: node_id.to_string(),
            kind,
            executor,
            ast,
            raw_script: Some(script.to_string()),
        })
    }

    fn check_return(&self, v: &Value) -> LFResult<Value> {
        let ok = match self.kind {
            ScriptKind::Common => true,
            ScriptKind::Boolean => v.is_boolean(),
            ScriptKind::Switch => v.is_string() || v.is_null(),
            ScriptKind::For => v.is_number(),
            ScriptKind::Iterator => v.is_array(),
        };
        if ok {
            Ok(v.clone())
        } else {
            let expect = match self.kind {
                ScriptKind::Boolean => "boolean",
                ScriptKind::Switch => "string",
                ScriptKind::For => "number",
                ScriptKind::Iterator => "array",
                ScriptKind::Common => unreachable!(),
            };
            Err(LiteflowError::NodeTypeError {
                node: self.node_id.clone(),
                expect: expect.into(),
                actual: v.to_string(),
            })
        }
    }
}

#[async_trait]
impl NodeComponent for ScriptComponent {
    async fn process(&self, ctx: &CmpContext) -> Result<Value, LiteflowError> {
        #[cfg(feature = "lua")]
        if let Some(script) = &self.raw_script {
            let v = crate::script::lua_executor::LuaScriptExecutor::execute(&self.node_id, script, ctx)?;
            return self.check_return(&v);
        }
        let v = self.executor.execute(&self.node_id, &self.ast, ctx)?;
        self.check_return(&v)
    }

    fn name(&self) -> &str {
        &self.node_id
    }
}
