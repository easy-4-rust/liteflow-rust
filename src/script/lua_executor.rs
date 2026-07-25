//! 对应 liteflow-script-lua：Lua 脚本引擎（mlua，feature "lua"）。
//! 注入全局变量与 rhai 版一致：input / data / node_id / tag / loop_index / loop_object；
//! data 为表，脚本结束后变更合并回上下文。

use crate::exception::{LFResult, LiteflowError};
use crate::slot::CmpContext;
use mlua::{Lua, Table, Value as LValue};
use serde_json::Value;

pub struct LuaScriptExecutor;

fn json_to_lua(lua: &Lua, v: &Value) -> mlua::Result<LValue> {
    Ok(match v {
        Value::Null => LValue::Nil,
        Value::Bool(b) => LValue::Boolean(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                LValue::Integer(i)
            } else {
                LValue::Number(n.as_f64().unwrap_or(0.0))
            }
        }
        Value::String(s) => LValue::String(lua.create_string(s)?),
        Value::Array(a) => {
            let t = lua.create_table()?;
            for (i, item) in a.iter().enumerate() {
                t.set(i + 1, json_to_lua(lua, item)?)?;
            }
            LValue::Table(t)
        }
        Value::Object(o) => {
            let t = lua.create_table()?;
            for (k, item) in o {
                t.set(k.as_str(), json_to_lua(lua, item)?)?;
            }
            LValue::Table(t)
        }
    })
}

fn lua_to_json(v: &LValue) -> Value {
    match v {
        LValue::Nil => Value::Null,
        LValue::Boolean(b) => Value::Bool(*b),
        LValue::Integer(i) => Value::from(*i),
        LValue::Number(f) => Value::from(*f),
        LValue::String(s) => Value::String(s.to_str().map(|x| x.to_string()).unwrap_or_default()),
        LValue::Table(t) => {
            // 连续整数键视为数组
            let len = t.raw_len();
            if len > 0 {
                let mut arr = Vec::with_capacity(len);
                for i in 1..=len {
                    arr.push(lua_to_json(&t.raw_get(i).unwrap_or(LValue::Nil)));
                }
                Value::Array(arr)
            } else {
                let mut obj = serde_json::Map::new();
                for pair in t.clone().pairs::<LValue, LValue>().flatten() {
                    if let LValue::String(k) = pair.0 {
                        obj.insert(k.to_str().map(|x| x.to_string()).unwrap_or_default(), lua_to_json(&pair.1));
                    }
                }
                Value::Object(obj)
            }
        }
        other => Value::String(other.to_string().unwrap_or_default()),
    }
}

impl LuaScriptExecutor {
    /// 对应 ScriptExecutor.execute（Lua 版）
    pub fn execute(node_id: &str, script: &str, ctx: &CmpContext) -> LFResult<Value> {
        let lua = Lua::new();
        let globals = lua.globals();

        let input = ctx.inner.input.lock().map(|v| v.clone()).unwrap_or(Value::Null);
        let to_l = |v: &Value| -> mlua::Result<LValue> { json_to_lua(&lua, v) };

        let set = |k: &str, v: LValue| -> LFResult<()> {
            globals.set(k, v).map_err(|e| LiteflowError::Script {
                node: node_id.to_string(),
                msg: format!("inject error: {e}"),
            })
        };

        set("input", to_l(&input).map_err(|e| LiteflowError::Script { node: node_id.into(), msg: e.to_string() })?)?;
        set("node_id", to_l(&Value::from(ctx.node.id.clone())).map_err(|e| LiteflowError::Script { node: node_id.into(), msg: e.to_string() })?)?;
        let tag_v = ctx.node.tag.clone().map(Value::from).unwrap_or(Value::Null);
        set("tag", to_l(&tag_v).map_err(|e| LiteflowError::Script { node: node_id.into(), msg: e.to_string() })?)?;
        let li_v = ctx.frame.loop_index().map(|i| Value::from(i as i64)).unwrap_or(Value::Null);
        set("loop_index", to_l(&li_v).map_err(|e| LiteflowError::Script { node: node_id.into(), msg: e.to_string() })?)?;
        let lo_v = ctx.frame.loop_object().cloned().unwrap_or(Value::Null);
        set("loop_object", to_l(&lo_v).map_err(|e| LiteflowError::Script { node: node_id.into(), msg: e.to_string() })?)?;

        // data 表：快照注入
        let data_table: Table = lua.create_table().map_err(|e| LiteflowError::Script { node: node_id.into(), msg: e.to_string() })?;
        for r in ctx.inner.data.iter() {
            data_table
                .set(r.key().as_str(), to_l(r.value()).map_err(|e| LiteflowError::Script { node: node_id.into(), msg: e.to_string() })?)
                .map_err(|e| LiteflowError::Script { node: node_id.into(), msg: e.to_string() })?;
        }
        set("data", LValue::Table(data_table))?;

        let result: LValue = lua
            .load(script)
            .eval()
            .map_err(|e| LiteflowError::Script {
                node: node_id.to_string(),
                msg: format!("eval error: {e}"),
            })?;

        // data 变更合并回上下文
        let data_back: Table = globals.get("data").map_err(|e| LiteflowError::Script { node: node_id.into(), msg: e.to_string() })?;
        for pair in data_back.pairs::<LValue, LValue>().flatten() {
            if let LValue::String(k) = pair.0 {
                ctx.inner.data.insert(
                    k.to_str().map(|x| x.to_string()).unwrap_or_default(),
                    lua_to_json(&pair.1),
                );
            }
        }

        Ok(lua_to_json(&result))
    }
}
