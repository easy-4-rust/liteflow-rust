//! 对应 parser.el.LocalJsonFlowELParser / JsonFlowELParser。
//! 兼容 LiteFlow 标准 JSON 规则格式（含 nodes/route/body/namespace/enable/extends）。

use super::chain_def::{resolve_and_build, ChainDef};
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::chain::DEFAULT_NAMESPACE;
use crate::flow::flow_bus::FlowBus;
use crate::script::script_component::ScriptKind;
use serde_json::Value;
use std::path::Path;

/// 从 serde_json::Value 解析（YML 解析器复用此入口）
pub fn load_value(bus: &FlowBus, v: &Value) -> LFResult<Vec<String>> {
    let flow = v
        .get("flow")
        .ok_or_else(|| LiteflowError::Rule("missing flow".into()))?;

    // flow.nodes.node：脚本节点
    if let Some(nodes) = flow
        .get("nodes")
        .and_then(|n| n.get("node"))
        .and_then(|n| n.as_array())
    {
        for node in nodes {
            let id = node
                .get("id")
                .and_then(|x| x.as_str())
                .ok_or_else(|| LiteflowError::Rule("node missing id".into()))?;
            let kind = node
                .get("type")
                .and_then(|x| x.as_str())
                .map(ScriptKind::from_code)
                .unwrap_or(Some(ScriptKind::Common))
                .ok_or_else(|| LiteflowError::Rule(format!("node[{id}] unsupported type")))?;
            let language = node.get("language").and_then(|x| x.as_str()).unwrap_or("rhai");
            let script = node
                .get("script")
                .and_then(|x| x.as_str())
                .ok_or_else(|| LiteflowError::Rule(format!("node[{id}] missing script")))?;
            bus.register_script_typed(id, language, kind, script)?;
        }
    }

    let chains = flow
        .get("chain")
        .and_then(|c| c.as_array())
        .ok_or_else(|| LiteflowError::Rule("missing flow.chain".into()))?;

    let mut defs = Vec::new();
    for chain in chains {
        let id = chain
            .get("id")
            .or_else(|| chain.get("name"))
            .and_then(|x| x.as_str())
            .ok_or_else(|| LiteflowError::Rule("chain missing id/name".into()))?
            .to_string();
        let mut def = ChainDef::new(id.clone(), "");
        if let Some(ns) = chain.get("namespace").and_then(|x| x.as_str()) {
            def.namespace = ns.to_string();
        } else {
            def.namespace = DEFAULT_NAMESPACE.to_string();
        }
        if let Some(b) = chain.get("enable").and_then(|x| x.as_bool()) {
            def.enable = b;
        }
        def.extends = chain.get("extends").and_then(|x| x.as_str()).map(|s| s.to_string());
        def.route = chain.get("route").and_then(|x| x.as_str()).map(|s| s.to_string());

        // body 字段优先，否则 condition 数组拼接
        if let Some(b) = chain.get("body").and_then(|x| x.as_str()) {
            def.body = b.to_string();
        } else if def.route.is_some() {
            return Err(LiteflowError::Rule(format!(
                "If you have defined the field route, then you must define the field body in chain[{id}]"
            )));
        } else {
            let conditions = chain
                .get("condition")
                .and_then(|c| c.as_array())
                .ok_or_else(|| LiteflowError::Rule(format!("chain[{id}] missing condition")))?;
            let mut parts = Vec::new();
            for cond in conditions {
                let ctype = cond
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("then")
                    .to_ascii_uppercase();
                let value = cond.get("value").and_then(|x| x.as_str()).ok_or_else(|| {
                    LiteflowError::Rule(format!("chain[{id}] condition missing value"))
                })?;
                parts.push(format!("{ctype}({value})"));
            }
            def.body = if parts.len() == 1 {
                parts.into_iter().next().unwrap()
            } else {
                format!("THEN({})", parts.join(","))
            };
        }
        defs.push(def);
    }
    resolve_and_build(bus, defs)
}

/// parse 一个 JSON 文本
pub fn load_json_str(bus: &FlowBus, json: &str) -> LFResult<Vec<String>> {
    let v: Value =
        serde_json::from_str(json).map_err(|e| LiteflowError::Rule(format!("invalid json: {e}")))?;
    load_value(bus, &v)
}

/// 本地文件加载
pub fn load_json_file(bus: &FlowBus, path: impl AsRef<Path>) -> LFResult<Vec<String>> {
    let text = std::fs::read_to_string(path.as_ref())
        .map_err(|e| LiteflowError::Rule(format!("read rule file error: {e}")))?;
    load_json_str(bus, &text)
}
