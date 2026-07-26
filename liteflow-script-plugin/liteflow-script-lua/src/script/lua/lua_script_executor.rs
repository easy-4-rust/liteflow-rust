//! 对应 Java: `com.yomahub.liteflow.script.lua.LuaScriptExecutor`。
//!
//! 使用 mlua/Lua 5.4，注入 input/data/node_id/tag/loop_index/loop_object；
//! data 表的修改在脚本结束后合并回 LiteFlow 上下文。

use std::sync::Arc;

use async_trait::async_trait;
use liteflow_core::core::NodeComponent;
use liteflow_core::script::{ScriptExecutorFactory, ScriptKind};
use liteflow_core::{CmpContext, LFResult, LiteflowError};
use mlua::{Lua, Table, Value as LuaValue};
use serde_json::Value;

/// Lua 脚本执行组件。
pub struct LuaScriptExecutor {
    node_id: String,
    kind: ScriptKind,
    script: String,
}

impl LuaScriptExecutor {
    /// 向 core ScriptExecutorFactory 注册 `language = "lua"`。
    pub fn register() -> LFResult<()> {
        ScriptExecutorFactory::register("lua", Self::build)
    }

    fn build(node_id: &str, kind: ScriptKind, script: &str) -> LFResult<Arc<dyn NodeComponent>> {
        // 构建期做语法编译，对应 Java JSR223ScriptExecutor#load。
        let lua = Lua::new();
        lua.load(script)
            .into_function()
            .map_err(|error| LiteflowError::Script {
                node: node_id.to_string(),
                msg: format!("compile error: {error}"),
            })?;
        Ok(Arc::new(Self {
            node_id: node_id.to_string(),
            kind,
            script: script.to_string(),
        }))
    }

    /// 执行 Lua。对应 Java `ScriptExecutor#executeScript`。
    fn execute(&self, ctx: &CmpContext) -> LFResult<Value> {
        let lua = Lua::new();
        let globals = lua.globals();
        let input = ctx
            .inner
            .input
            .lock()
            .map(|value| value.clone())
            .unwrap_or(Value::Null);

        set_global(&lua, &globals, &self.node_id, "input", &input)?;
        set_global(
            &lua,
            &globals,
            &self.node_id,
            "node_id",
            &Value::String(ctx.node.id.clone()),
        )?;
        set_global(
            &lua,
            &globals,
            &self.node_id,
            "tag",
            &ctx.node
                .tag
                .clone()
                .map(Value::String)
                .unwrap_or(Value::Null),
        )?;
        set_global(
            &lua,
            &globals,
            &self.node_id,
            "loop_index",
            &ctx.frame
                .loop_index()
                .map(|index| Value::from(index as i64))
                .unwrap_or(Value::Null),
        )?;
        set_global(
            &lua,
            &globals,
            &self.node_id,
            "loop_object",
            &ctx.frame.loop_object().cloned().unwrap_or(Value::Null),
        )?;

        let data = lua.create_table().map_err(|error| self.error(error))?;
        for entry in ctx.inner.data.iter() {
            data.set(
                entry.key().as_str(),
                json_to_lua(&lua, entry.value()).map_err(|error| self.error(error))?,
            )
            .map_err(|error| self.error(error))?;
        }
        globals
            .set("data", data)
            .map_err(|error| self.error(error))?;

        let result: LuaValue = lua
            .load(&self.script)
            .eval()
            .map_err(|error| self.error(format!("eval error: {error}")))?;

        let data: Table = globals.get("data").map_err(|error| self.error(error))?;
        for pair in data.pairs::<LuaValue, LuaValue>() {
            let (key, value) = pair.map_err(|error| self.error(error))?;
            if let LuaValue::String(key) = key {
                ctx.inner.data.insert(
                    key.to_str().map(|key| key.to_string()).unwrap_or_default(),
                    lua_to_json(&value),
                );
            }
        }
        Ok(lua_to_json(&result))
    }

    fn error(&self, error: impl std::fmt::Display) -> LiteflowError {
        LiteflowError::Script {
            node: self.node_id.clone(),
            msg: error.to_string(),
        }
    }
}

#[async_trait]
impl NodeComponent for LuaScriptExecutor {
    async fn process(&self, ctx: &CmpContext) -> LFResult<Value> {
        self.kind.check_return(&self.node_id, self.execute(ctx)?)
    }

    fn name(&self) -> &str {
        &self.node_id
    }
}

fn set_global(lua: &Lua, globals: &Table, node_id: &str, key: &str, value: &Value) -> LFResult<()> {
    globals
        .set(
            key,
            json_to_lua(lua, value).map_err(|error| LiteflowError::Script {
                node: node_id.to_string(),
                msg: error.to_string(),
            })?,
        )
        .map_err(|error| LiteflowError::Script {
            node: node_id.to_string(),
            msg: format!("inject error: {error}"),
        })
}

fn json_to_lua(lua: &Lua, value: &Value) -> mlua::Result<LuaValue> {
    Ok(match value {
        Value::Null => LuaValue::Nil,
        Value::Bool(value) => LuaValue::Boolean(*value),
        Value::Number(value) => value
            .as_i64()
            .map(LuaValue::Integer)
            .unwrap_or_else(|| LuaValue::Number(value.as_f64().unwrap_or_default())),
        Value::String(value) => LuaValue::String(lua.create_string(value)?),
        Value::Array(values) => {
            let table = lua.create_table()?;
            for (index, value) in values.iter().enumerate() {
                table.set(index + 1, json_to_lua(lua, value)?)?;
            }
            LuaValue::Table(table)
        }
        Value::Object(values) => {
            let table = lua.create_table()?;
            for (key, value) in values {
                table.set(key.as_str(), json_to_lua(lua, value)?)?;
            }
            LuaValue::Table(table)
        }
    })
}

fn lua_to_json(value: &LuaValue) -> Value {
    match value {
        LuaValue::Nil => Value::Null,
        LuaValue::Boolean(value) => Value::Bool(*value),
        LuaValue::Integer(value) => Value::from(*value),
        LuaValue::Number(value) => Value::from(*value),
        LuaValue::String(value) => Value::String(
            value
                .to_str()
                .map(|value| value.to_string())
                .unwrap_or_default(),
        ),
        LuaValue::Table(table) if table.raw_len() > 0 => Value::Array(
            (1..=table.raw_len())
                .map(|index| lua_to_json(&table.raw_get(index).unwrap_or(LuaValue::Nil)))
                .collect(),
        ),
        LuaValue::Table(table) => {
            let mut object = serde_json::Map::new();
            for (key, value) in table.clone().pairs::<LuaValue, LuaValue>().flatten() {
                if let LuaValue::String(key) = key {
                    object.insert(
                        key.to_str().map(|key| key.to_string()).unwrap_or_default(),
                        lua_to_json(&value),
                    );
                }
            }
            Value::Object(object)
        }
        other => Value::String(other.to_string().unwrap_or_default()),
    }
}
