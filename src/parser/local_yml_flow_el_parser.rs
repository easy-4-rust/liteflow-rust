//! 对应 parser.el.LocalYmlFlowELParser / YmlFlowELParser。
//! YML 与 JSON 共用 schema（serde_yaml → serde_json::Value → load_value）。

use super::local_json_flow_el_parser::load_value;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::flow_bus::FlowBus;
use std::path::Path;

/// parse 一个 YML 文本
pub fn load_yml_str(bus: &FlowBus, yml: &str) -> LFResult<Vec<String>> {
    let v: serde_yaml::Value = serde_yaml::from_str(yml)
        .map_err(|e| LiteflowError::Rule(format!("invalid yml: {e}")))?;
    let j: serde_json::Value = serde_json::to_value(v)
        .map_err(|e| LiteflowError::Rule(format!("yml convert error: {e}")))?;
    load_value(bus, &j)
}

/// 本地 YML 文件加载
pub fn load_yml_file(bus: &FlowBus, path: impl AsRef<Path>) -> LFResult<Vec<String>> {
    let text = std::fs::read_to_string(path.as_ref())
        .map_err(|e| LiteflowError::Rule(format!("read rule file error: {e}")))?;
    load_yml_str(bus, &text)
}
