//! 对应 parser.el.LocalXmlFlowELParser / XmlFlowELParser。
//!
//! 兼容 LiteFlow 标准 XML 规则格式（EL 模式）：
//! ```xml
//! <flow>
//!   <nodes>
//!     <node id="s1" type="boolean_script" language="rhai"><![CDATA[input > 3]]></node>
//!   </nodes>
//!   <chain name="chain1">THEN(a, WHEN(b, c))</chain>
//!   <chain name="route1" namespace="ns1">
//!     <route>IF(r1)</route>
//!     <body>THEN(a, b)</body>
//!   </chain>
//! </flow>
//! ```
//! 对齐 Java ParserHelper.parseChainForEL 的字段语义：
//! - id / name 属性（id 优先）
//! - namespace 属性（默认 DEFAULT）
//! - enable="false" 跳过该 chain
//! - 有 <route> 必须有 <body>，否则报错
//! - 无 <route> 时 <body> 可省略，EL 直接写在 chain 文本里
//! - extends 属性（链继承）暂未迁移，见迁移对照表

use super::chain_def::{resolve_and_build, ChainDef};
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::chain::DEFAULT_NAMESPACE;
use crate::flow::flow_bus::FlowBus;
use crate::script::script_component::ScriptKind;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::path::Path;

fn rule_err(msg: impl Into<String>) -> LiteflowError {
    LiteflowError::Rule(msg.into())
}

/// parse XML 文本，返回加载的 chain id 列表
pub fn load_xml_str(bus: &FlowBus, xml: &str) -> LFResult<Vec<String>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut defs: Vec<ChainDef> = Vec::new();
    let mut buf = Vec::new();

    let chain_ids;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag == "chain" {
                    let (id, ns, enable, extends) = parse_chain_attrs(&e)?;
                    if enable == Some(false) {
                        skip_element(&mut reader, "chain")?;
                        continue;
                    }
                    let id = id.ok_or_else(|| rule_err("missing chain id in expression"))?;
                    let (route, body) = read_chain_content(&mut reader, &id)?;
                    let mut def = ChainDef::new(id.clone(), body);
                    def.namespace = ns.unwrap_or_else(|| DEFAULT_NAMESPACE.to_string());
                    def.route = route;
                    def.extends = extends;
                    defs.push(def);
                } else if tag == "nodes" {
                    read_nodes(bus, &mut reader)?;
                } else if tag == "flow" {
                    // 根元素，继续向下解析
                } else {
                    skip_element(&mut reader, &tag)?;
                }
            }
            Ok(Event::Empty(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag == "chain" {
                    let (id, _, _, _) = parse_chain_attrs(&e)?;
                    let id = id.ok_or_else(|| rule_err("missing chain id in expression"))?;
                    return Err(rule_err(format!("chain[{id}] has empty EL")));
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(rule_err(format!("xml parse error: {e}"))),
            _ => {}
        }
        buf.clear();
    }
    chain_ids = resolve_and_build(bus, defs)?;
    Ok(chain_ids)
}

type Attrs = (Option<String>, Option<String>, Option<bool>, Option<String>);

fn parse_chain_attrs(e: &quick_xml::events::BytesStart) -> LFResult<Attrs> {
    let mut id = None;
    let mut ns = None;
    let mut enable = None;
    let mut extends = None;
    for attr in e.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
        let val = String::from_utf8_lossy(&attr.value).to_string();
        match key.as_str() {
            "id" => id = Some(val),
            "name" => {
                if id.is_none() {
                    id = Some(val)
                }
            }
            "namespace" => ns = Some(val),
            "enable" => enable = Some(!val.eq_ignore_ascii_case("false")),
            "extends" => extends = Some(val),
            _ => {}
        }
    }
    Ok((id, ns, enable, extends))
}

/// 读取 chain 内容：<route>/<body> 子元素或直接文本（对齐 ParserHelper 语义）
fn read_chain_content(
    reader: &mut Reader<&[u8]>,
    chain_id: &str,
) -> LFResult<(Option<String>, String)> {
    let mut route = None;
    let mut body = None;
    let mut text = String::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let content = read_text_until(reader, &tag)?;
                match tag.as_str() {
                    "route" => route = Some(content),
                    "body" => body = Some(content),
                    _ => {}
                }
            }
            Ok(Event::Text(t)) => text.push_str(&String::from_utf8_lossy(&t)),
            Ok(Event::CData(t)) => text.push_str(&String::from_utf8_lossy(&t)),
            Ok(Event::End(e)) if e.name().as_ref() == b"chain" => break,
            Ok(Event::Eof) => return Err(rule_err(format!("chain[{chain_id}] unclosed"))),
            Err(e) => return Err(rule_err(format!("xml parse error: {e}"))),
            _ => {}
        }
        buf.clear();
    }

    match route {
        Some(r) => {
            // 有 route 必须有 body
            let b = body.ok_or_else(|| {
                rule_err(format!(
                    "If you have defined the tag <route>, then you must define the tag <body> in chain[{chain_id}]"
                ))
            })?;
            Ok((Some(r.trim().to_string()), b.trim().to_string()))
        }
        None => {
            let el = match body {
                Some(b) => b.trim().to_string(),
                None => text.trim().to_string(),
            };
            if el.is_empty() {
                return Err(rule_err(format!("chain[{chain_id}] has empty EL")));
            }
            Ok((None, el))
        }
    }
}

/// 读取元素文本直到闭合标签
fn read_text_until(reader: &mut Reader<&[u8]>, tag: &str) -> LFResult<String> {
    let mut text = String::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Text(t)) => text.push_str(&String::from_utf8_lossy(&t)),
            Ok(Event::CData(t)) => text.push_str(&String::from_utf8_lossy(&t)),
            Ok(Event::End(e)) if e.name().as_ref() == tag.as_bytes() => break,
            Ok(Event::Eof) => return Err(rule_err(format!("unclosed <{tag}>"))),
            Err(e) => return Err(rule_err(format!("xml parse error: {e}"))),
            _ => {}
        }
        buf.clear();
    }
    Ok(text)
}

/// 跳过整个元素
fn skip_element(reader: &mut Reader<&[u8]>, tag: &str) -> LFResult<()> {
    let mut depth = 1usize;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == tag.as_bytes() => depth += 1,
            Ok(Event::End(e)) if e.name().as_ref() == tag.as_bytes() => {
                depth -= 1;
                if depth == 0 {
                    return Ok(());
                }
            }
            Ok(Event::Eof) => return Ok(()),
            Err(e) => return Err(rule_err(format!("xml parse error: {e}"))),
            _ => {}
        }
        buf.clear();
    }
}

/// 解析 <nodes>：脚本节点（对应 NodeConvertHelper 的脚本节点语义）
fn read_nodes(bus: &FlowBus, reader: &mut Reader<&[u8]>) -> LFResult<()> {
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag != "node" {
                    skip_element(reader, &tag)?;
                    continue;
                }
                let mut id = None;
                let mut ntype = None;
                let mut language = None;
                for attr in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    let val = String::from_utf8_lossy(&attr.value).to_string();
                    match key.as_str() {
                        "id" => id = Some(val),
                        "type" => ntype = Some(val),
                        "language" => language = Some(val),
                        _ => {}
                    }
                }
                let id = id.ok_or_else(|| rule_err("node missing id"))?;
                let kind = ScriptKind::from_code(ntype.as_deref().unwrap_or("script"))
                    .ok_or_else(|| rule_err(format!("node[{id}] unsupported type")))?;
                let script = read_text_until(reader, "node")?;
                bus.register_script_typed(
                    id,
                    language.as_deref().unwrap_or("rhai"),
                    kind,
                    script.trim(),
                )?;
            }
            Ok(Event::End(e)) if e.name().as_ref() == b"nodes" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(rule_err(format!("xml parse error: {e}"))),
            _ => {}
        }
        buf.clear();
    }
    Ok(())
}

/// 本地 XML 文件加载
pub fn load_xml_file(bus: &FlowBus, path: impl AsRef<Path>) -> LFResult<Vec<String>> {
    let text = std::fs::read_to_string(path.as_ref())
        .map_err(|e| LiteflowError::Rule(format!("read rule file error: {e}")))?;
    load_xml_str(bus, &text)
}
