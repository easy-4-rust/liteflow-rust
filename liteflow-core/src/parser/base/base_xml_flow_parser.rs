//! XML 规则解析器公共实现。

use crate::builder::NodePropBean;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::chain::DEFAULT_NAMESPACE;
use crate::flow::flow_bus::FlowBus;
use crate::parser::RuleDefinitionPlan;
use crate::parser::chain_def::ChainDef;
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

/// 承载 XML/XML-EL 解析器共享的节点与链路解析逻辑。
///
/// 多份 XML 文本共享同一批中间链定义，最后统一解析继承并原子写入
/// `FlowBus`，避免跨文件父子链被拆开处理。
///
/// 对应 Java: `com.yomahub.liteflow.parser.base.BaseXmlFlowParser`。
#[derive(Clone)]
pub struct BaseXmlFlowParser {
    bus: FlowBus,
}

impl BaseXmlFlowParser {
    /// 使用目标流程总线创建解析器。
    #[must_use]
    pub fn new(bus: FlowBus) -> Self {
        Self { bus }
    }

    /// 解析 XML 文本列表并返回成功装载的 chain id。
    ///
    /// 对应 Java: `BaseXmlFlowParser#parse(List<String>)`。
    pub fn parse(&self, content_list: &[String]) -> LFResult<Vec<String>> {
        if content_list.is_empty() {
            return Ok(Vec::new());
        }

        self.collect(content_list)?.build_all(&self.bus)
    }

    /// 只读取 XML 节点与链定义，不创建 Chain 或编译脚本。
    ///
    /// 对应 Java `PARSE_ONE_ON_FIRST_EXEC` 的启动期定义收集阶段。
    pub fn collect(&self, content_list: &[String]) -> LFResult<RuleDefinitionPlan> {
        let mut plan = RuleDefinitionPlan::new();
        for content in content_list {
            collect_document(content, &mut plan)?;
        }
        Ok(plan)
    }
}

fn rule_error(message: impl Into<String>) -> LiteflowError {
    LiteflowError::Rule(message.into())
}

fn collect_document(content: &str, plan: &mut RuleDefinitionPlan) -> LFResult<()> {
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(element)) => {
                let tag = String::from_utf8_lossy(element.name().as_ref()).to_string();
                match tag.as_str() {
                    "chain" => {
                        let (id, namespace, enabled, extends) = parse_chain_attrs(&element);
                        if enabled == Some(false) {
                            skip_element(&mut reader, "chain")?;
                            continue;
                        }
                        let id = id.ok_or_else(|| rule_error("missing chain id in expression"))?;
                        let (route, body) = read_chain_content(&mut reader, &id)?;
                        let mut definition = ChainDef::new(id, body);
                        definition.namespace =
                            namespace.unwrap_or_else(|| DEFAULT_NAMESPACE.to_string());
                        definition.route = route;
                        definition.extends = extends;
                        plan.push_chain(definition);
                    }
                    "nodes" => read_nodes(&mut reader, plan)?,
                    "flow" => {}
                    _ => skip_element(&mut reader, &tag)?,
                }
            }
            Ok(Event::Empty(element)) if element.name().as_ref() == b"chain" => {
                let (id, _, _, _) = parse_chain_attrs(&element);
                let id = id.ok_or_else(|| rule_error("missing chain id in expression"))?;
                return Err(rule_error(format!("chain[{id}] has empty EL")));
            }
            Ok(Event::Eof) => return Ok(()),
            Err(error) => return Err(rule_error(format!("xml parse error: {error}"))),
            _ => {}
        }
        buffer.clear();
    }
}

type ChainAttrs = (Option<String>, Option<String>, Option<bool>, Option<String>);

fn parse_chain_attrs(element: &BytesStart<'_>) -> ChainAttrs {
    let mut id = None;
    let mut namespace = None;
    let mut enabled = None;
    let mut extends = None;
    for attribute in element.attributes().flatten() {
        let key = String::from_utf8_lossy(attribute.key.as_ref());
        let value = String::from_utf8_lossy(&attribute.value).to_string();
        match key.as_ref() {
            "id" => id = Some(value),
            "name" if id.is_none() => id = Some(value),
            "namespace" => namespace = Some(value),
            "enable" => enabled = Some(!value.eq_ignore_ascii_case("false")),
            "extends" => extends = Some(value),
            _ => {}
        }
    }
    (id, namespace, enabled, extends)
}

fn read_chain_content(
    reader: &mut Reader<&[u8]>,
    chain_id: &str,
) -> LFResult<(Option<String>, String)> {
    let mut route = None;
    let mut body = None;
    let mut direct_text = String::new();
    let mut buffer = Vec::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(element)) => {
                let tag = String::from_utf8_lossy(element.name().as_ref()).to_string();
                let content = read_text_until(reader, &tag)?;
                match tag.as_str() {
                    "route" => route = Some(content),
                    "body" => body = Some(content),
                    _ => {}
                }
            }
            Ok(Event::Text(text)) => direct_text.push_str(&String::from_utf8_lossy(&text)),
            Ok(Event::CData(text)) => direct_text.push_str(&String::from_utf8_lossy(&text)),
            Ok(Event::End(element)) if element.name().as_ref() == b"chain" => break,
            Ok(Event::Eof) => return Err(rule_error(format!("chain[{chain_id}] unclosed"))),
            Err(error) => return Err(rule_error(format!("xml parse error: {error}"))),
            _ => {}
        }
        buffer.clear();
    }

    if let Some(route) = route {
        let body = body.ok_or_else(|| {
            rule_error(format!(
                "If you have defined the tag <route>, then you must define the tag <body> in chain[{chain_id}]"
            ))
        })?;
        return Ok((Some(route.trim().to_string()), body.trim().to_string()));
    }

    let expression = body.unwrap_or(direct_text).trim().to_string();
    if expression.is_empty() {
        return Err(rule_error(format!("chain[{chain_id}] has empty EL")));
    }
    Ok((None, expression))
}

fn read_text_until(reader: &mut Reader<&[u8]>, tag: &str) -> LFResult<String> {
    let mut content = String::new();
    let mut buffer = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Text(text)) => content.push_str(&String::from_utf8_lossy(&text)),
            Ok(Event::CData(text)) => content.push_str(&String::from_utf8_lossy(&text)),
            Ok(Event::End(element)) if element.name().as_ref() == tag.as_bytes() => {
                return Ok(content);
            }
            Ok(Event::Eof) => return Err(rule_error(format!("unclosed <{tag}>"))),
            Err(error) => return Err(rule_error(format!("xml parse error: {error}"))),
            _ => {}
        }
        buffer.clear();
    }
}

fn skip_element(reader: &mut Reader<&[u8]>, tag: &str) -> LFResult<()> {
    let mut depth = 1usize;
    let mut buffer = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(element)) if element.name().as_ref() == tag.as_bytes() => depth += 1,
            Ok(Event::End(element)) if element.name().as_ref() == tag.as_bytes() => {
                depth -= 1;
                if depth == 0 {
                    return Ok(());
                }
            }
            Ok(Event::Eof) => return Ok(()),
            Err(error) => return Err(rule_error(format!("xml parse error: {error}"))),
            _ => {}
        }
        buffer.clear();
    }
}

fn read_nodes(reader: &mut Reader<&[u8]>, plan: &mut RuleDefinitionPlan) -> LFResult<()> {
    let mut buffer = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(element)) => {
                let tag = String::from_utf8_lossy(element.name().as_ref()).to_string();
                if tag != "node" {
                    skip_element(reader, &tag)?;
                    continue;
                }
                let (mut property, enabled) = parse_node_attrs(&element);
                let script = read_text_until(reader, "node")?;
                if !script.trim().is_empty() {
                    property.script = Some(script.trim().to_string());
                }
                if enabled {
                    plan.push_node(property);
                }
            }
            Ok(Event::Empty(element)) if element.name().as_ref() == b"node" => {
                let (property, enabled) = parse_node_attrs(&element);
                if enabled {
                    plan.push_node(property);
                }
            }
            Ok(Event::End(element)) if element.name().as_ref() == b"nodes" => return Ok(()),
            Ok(Event::Eof) => return Ok(()),
            Err(error) => return Err(rule_error(format!("xml parse error: {error}"))),
            _ => {}
        }
        buffer.clear();
    }
}

fn parse_node_attrs(element: &BytesStart<'_>) -> (NodePropBean, bool) {
    let mut property = NodePropBean::default();
    let mut enabled = true;
    for attribute in element.attributes().flatten() {
        let key = String::from_utf8_lossy(attribute.key.as_ref());
        let value = String::from_utf8_lossy(&attribute.value).to_string();
        match key.as_ref() {
            "id" => property.id = Some(value),
            "name" => property.name = Some(value),
            "class" | "clazz" => property.clazz = Some(value),
            "type" => property.node_type = Some(value),
            "file" => property.file = Some(value),
            "language" => property.language = Some(value),
            "value" | "script" => property.script = Some(value),
            "enable" => enabled = !value.eq_ignore_ascii_case("false"),
            _ => {}
        }
    }
    (property, enabled)
}
