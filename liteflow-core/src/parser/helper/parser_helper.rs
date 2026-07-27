use std::collections::{HashMap, HashSet};

use crate::builder::el::lite_flow_chain_el_builder::LiteFlowChainELBuilder;
use crate::builder::{LiteFlowNodeBuilder, NodePropBean};
use crate::el::{El, parse_el};
use crate::enums::NodeTypeEnum;
use crate::exception::{ChainDuplicateException, LFResult, LiteflowError};
use crate::flow::element::chain::DEFAULT_NAMESPACE;
use crate::flow::flow_bus::FlowBus;
use crate::script::ScriptKind;
use crate::util::el_regex_util::{is_abstract_chain, replace_abstract_chain};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use serde_json::Value;

/// `ParserHelper` 解析阶段使用的链定义伴随类型。
///
/// 它不是额外的 Java 对象，只保存 Java `parseOneChain` 创建
/// `LiteFlowChainELBuilder` 前读取的字段，因此与主对象放在同一文件。
#[derive(Debug, Clone)]
pub struct ChainDef {
    /// Chain ID。
    pub id: String,
    /// Chain 命名空间。
    pub namespace: String,
    /// 决策路由表达式。
    pub route: Option<String>,
    /// Chain 主体表达式。
    pub body: String,
    /// 抽象父 Chain ID。
    pub extends: Option<String>,
    /// Chain 级线程池执行器构建器类名或 Rust 注册键。
    pub thread_pool_executor_class: Option<String>,
    /// 是否启用。
    pub enable: bool,
}

impl ChainDef {
    fn new(id: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            namespace: DEFAULT_NAMESPACE.to_string(),
            route: None,
            body: body.into(),
            extends: None,
            thread_pool_executor_class: None,
            enable: true,
        }
    }
}

/// `ParserHelper` 的延迟物化伴随计划。
///
/// Java `PARSE_ONE_ON_FIRST_EXEC` 会先保存 Chain 定义，首次执行时才构建相关
/// Chain 与脚本节点。Rust 用该伴随类型显式保存同一状态；它不是独立 Java
/// 对象，因此与 `ParserHelper` 同文件。
#[derive(Debug, Clone, Default)]
pub struct RuleDefinitionPlan {
    nodes: Vec<NodePropBean>,
    chains: Vec<ChainDef>,
}

impl RuleDefinitionPlan {
    /// 创建空规则计划。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加一个已通过格式解析的节点定义。
    ///
    /// 参数 `node` 是 `parseNodeDocument/parseNodeJson` 提取的中间属性。
    pub fn push_node(&mut self, node: NodePropBean) {
        self.nodes.push(node);
    }

    /// 追加一个已通过格式解析的链定义。
    ///
    /// 参数 `chain` 是 `parseOneChain` 提取的伴随定义。
    pub fn push_chain(&mut self, chain: ChainDef) {
        self.chains.push(chain);
    }

    /// 返回计划中启用的可执行或抽象链数量。
    #[must_use]
    pub fn chain_count(&self) -> usize {
        self.chains.iter().filter(|chain| chain.enable).count()
    }

    /// 立即构建全部节点与全部可执行链。
    ///
    /// 参数 `bus` 接收真实节点、脚本执行器和 Chain；返回成功注册的 Chain ID。
    /// 对应 Java: `ParserHelper#parseChainDocument`/`parseChainJson` 的编译阶段。
    pub fn build_all(&self, bus: &FlowBus) -> LFResult<Vec<String>> {
        for node in self.nodes.iter().cloned() {
            ParserHelper::build_node(bus, node)?;
        }
        let chain_ids: Vec<String> = self
            .chains
            .iter()
            .filter(|chain| {
                chain.enable && !(chain.extends.is_none() && is_abstract_chain(&chain.body))
            })
            .map(|chain| chain.id.clone())
            .collect();
        // 逐个入口按依赖拓扑构建，声明顺序不决定子链能否被解析。
        for chain_id in &chain_ids {
            self.build_chain(bus, chain_id)?;
        }
        Ok(chain_ids)
    }

    /// 仅构建指定 Chain 及其递归依赖的子链和节点。
    ///
    /// 参数 `bus` 是目标流程总线，`chain_id` 对应 Java 执行入口的 Chain ID。
    /// 未被依赖的脚本节点不会提前编译。
    /// 对应 Java: `ParseModeEnum#PARSE_ONE_ON_FIRST_EXEC`。
    pub fn build_chain(&self, bus: &FlowBus, chain_id: &str) -> LFResult<()> {
        if bus.contains_chain(chain_id) {
            return Ok(());
        }
        let definitions: HashMap<String, ChainDef> = self
            .chains
            .iter()
            .filter(|chain| chain.enable)
            .map(|chain| (chain.id.clone(), chain.clone()))
            .collect();
        let nodes: HashMap<String, NodePropBean> = self
            .nodes
            .iter()
            .filter_map(|node| node.id.clone().map(|id| (id, node.clone())))
            .collect();
        build_chain_recursive(
            bus,
            chain_id,
            &definitions,
            &nodes,
            &mut HashMap::new(),
            &mut HashSet::new(),
            &mut HashSet::new(),
            &mut HashSet::new(),
        )
    }
}

/// 规则解析器共享的节点校验与构建助手。
///
/// 统一承载 XML/JSON 节点与链路遍历、链定义提取、节点类型校验和真实注册。
/// `BaseXmlFlowParser`、`BaseJsonFlowParser` 和 `BaseYmlFlowParser` 都通过本
/// 对象进入相同的两阶段构建流程，避免同一 Java 对象的逻辑散落在多个文件。
///
/// 对应 Java: `com.yomahub.liteflow.parser.helper.ParserHelper`。
pub struct ParserHelper;

impl ParserHelper {
    /// 校验节点中间属性并将节点真实注册到流程总线。
    ///
    /// 参数 `bus` 是目标流程总线，`node_prop_bean` 对应 Java 构建中间对象；
    /// 成功返回空值，失败返回具体的类缺失、类型不可推断、类型不支持或构建错误。
    /// 对应 Java: `ParserHelper#buildNode`。
    pub fn build_node(bus: &FlowBus, mut node_prop_bean: NodePropBean) -> LFResult<()> {
        let id = node_prop_bean.id.clone().unwrap_or_default();

        // Rust 不支持 Java Class.forName；应用预注册的同 id 组件是 class 节点的
        // 可执行映射。未注册时明确报告边界，不能把类名伪装成已迁移组件。
        if node_prop_bean
            .clazz
            .as_deref()
            .is_some_and(|clazz| !clazz.trim().is_empty())
        {
            if bus.contains_node(&id) {
                node_prop_bean.node_type = Some(NodeTypeEnum::Common.get_code().to_string());
            } else {
                let clazz = node_prop_bean.clazz.as_deref().unwrap_or_default();
                return Err(LiteflowError::NodeClassNotFound(format!(
                    "cannot find the node[{clazz}]"
                )));
            }
        }

        // iterator_script 是早期 Rust 规则格式；保留其真实执行语义，但不将其
        // 混入 Java NodeTypeEnum 对照枚举。
        if node_prop_bean.node_type.as_deref() == Some("iterator_script") {
            let script = node_prop_bean
                .script
                .as_deref()
                .ok_or_else(|| LiteflowError::NodeBuild(format!("node[{id}] missing script")))?;
            return bus.register_script_typed(
                id,
                node_prop_bean.language.as_deref().unwrap_or("rhai"),
                ScriptKind::Iterator,
                script,
            );
        }

        let node_type = node_prop_bean
            .node_type
            .as_deref()
            .filter(|node_type| !node_type.trim().is_empty())
            .ok_or_else(|| {
                LiteflowError::NodeTypeCanNotGuess(format!(
                    "cannot guess the type of node[{}]",
                    node_prop_bean.clazz.as_deref().unwrap_or_default()
                ))
            })?;
        if NodeTypeEnum::get_enum_by_code(node_type.trim()).is_none() {
            return Err(LiteflowError::NodeTypeNotSupport(format!(
                "type [{}] is not support",
                node_type.trim()
            )));
        }

        LiteFlowNodeBuilder::from_prop(bus, node_prop_bean)?.build()
    }

    /// 从 JSON/YAML 转换后的文档中提取并保存节点定义。
    ///
    /// - `flow_json_object_list`：对应 Java `flowJsonObjectList`；
    /// - `plan`：接收启用的节点定义，节点尚不会在 `FlowBus` 中物化；
    /// - 返回：格式和 serde 映射均成功时返回 `Ok(())`。
    ///
    /// `enable=false` 的节点不会进入构建计划。Jackson 字段映射由 serde 的
    /// `class`/`value` rename 与 alias 完成。
    /// 对应 Java: `ParserHelper#parseNodeJson`。
    pub fn parse_node_json(
        flow_json_object_list: &[Value],
        plan: &mut RuleDefinitionPlan,
    ) -> LFResult<()> {
        for flow_json_object in flow_json_object_list {
            let flow = flow_json_object
                .get("flow")
                .ok_or_else(|| rule_error("missing flow"))?;
            let Some(nodes) = flow
                .get("nodes")
                .and_then(|nodes| nodes.get("node"))
                .and_then(Value::as_array)
            else {
                continue;
            };

            for node in nodes {
                if is_disabled(node.get("enable")) {
                    continue;
                }
                let id = node.get("id").and_then(Value::as_str).unwrap_or_default();
                let node_prop_bean = serde_json::from_value(node.clone())
                    .map_err(|error| rule_error(format!("invalid node[{id}] property: {error}")))?;
                plan.push_node(node_prop_bean);
            }
        }
        Ok(())
    }

    /// 从 JSON/YAML 转换后的文档中提取链定义。
    ///
    /// - `flow_json_object_list`：对应 Java `flowJsonObjectList`；
    /// - `chain_id_set`：跨文档检测重复链 ID，对应 Java 同名参数；
    /// - `plan`：接收尚未编译的链定义；
    /// - 返回：成功时返回 `Ok(())`，重复链或非法 route/body 返回明确错误。
    ///
    /// 所有文档先进入计划，再由计划统一处理抽象链继承并编译，保留 Java
    /// “先登记、后编译”的顺序。
    /// 对应 Java: `ParserHelper#parseChainJson`。
    pub fn parse_chain_json(
        flow_json_object_list: &[Value],
        chain_id_set: &mut HashSet<String>,
        plan: &mut RuleDefinitionPlan,
    ) -> LFResult<()> {
        for flow_json_object in flow_json_object_list {
            let Some(chains) = flow_json_object
                .get("flow")
                .and_then(|flow| flow.get("chain"))
                .and_then(Value::as_array)
            else {
                continue;
            };
            for chain_node in chains {
                let Some(chain) = Self::parse_one_chain(chain_node)? else {
                    continue;
                };
                if !chain_id_set.insert(chain.id.clone()) {
                    return Err(ChainDuplicateException::new(format!(
                        "[chain id duplicate] chainId={}",
                        chain.id
                    ))
                    .into());
                }
                plan.push_chain(chain);
            }
        }
        chain_id_set.clear();
        Ok(())
    }

    /// 把一条 JSON 链记录转换为格式无关的链定义。
    ///
    /// - `chain_node`：对应 Java `chainNode`；
    /// - 返回：禁用链返回 `Ok(None)`，启用链返回包含 namespace、route、
    ///   extends 和线程池类名的 `ChainDef`。
    ///
    /// route 存在时 body 必填；普通链既接受 Java EL 的 `body`，也保留旧规则
    /// `condition` 数组到 EL 的兼容转换。
    /// 对应 Java: `ParserHelper#parseOneChain(JsonNode)`。
    pub fn parse_one_chain(chain_node: &Value) -> LFResult<Option<ChainDef>> {
        if is_disabled(chain_node.get("enable")) {
            return Ok(None);
        }
        let id = chain_node
            .get("id")
            .or_else(|| chain_node.get("name"))
            .and_then(Value::as_str)
            .ok_or_else(|| rule_error("chain missing id/name"))?
            .to_string();
        let mut definition = ChainDef::new(id.clone(), "");
        definition.namespace = chain_node
            .get("namespace")
            .and_then(Value::as_str)
            .filter(|namespace| !namespace.trim().is_empty())
            .unwrap_or(DEFAULT_NAMESPACE)
            .to_string();
        definition.extends = chain_node
            .get("extends")
            .and_then(Value::as_str)
            .map(str::to_string);
        definition.route = chain_node
            .get("route")
            .and_then(Value::as_str)
            .map(str::to_string);
        definition.thread_pool_executor_class = chain_node
            .get("threadPoolExecutorClass")
            .and_then(Value::as_str)
            .map(str::to_string);

        if let Some(body) = chain_node.get("body").and_then(Value::as_str) {
            definition.body = body.to_string();
        } else if definition.route.is_some() {
            return Err(rule_error(format!(
                "If you have defined the field route, then you must define the field body in chain[{id}]"
            )));
        } else {
            let conditions = chain_node
                .get("condition")
                .and_then(Value::as_array)
                .ok_or_else(|| rule_error(format!("chain[{id}] missing condition")))?;
            let mut parts = Vec::with_capacity(conditions.len());
            for condition in conditions {
                let condition_type = condition
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("then")
                    .to_ascii_uppercase();
                let value = condition
                    .get("value")
                    .and_then(Value::as_str)
                    .ok_or_else(|| rule_error(format!("chain[{id}] condition missing value")))?;
                parts.push(format!("{condition_type}({value})"));
            }
            definition.body = if parts.len() == 1 {
                parts.remove(0)
            } else {
                format!("THEN({})", parts.join(","))
            };
        }
        Ok(Some(definition))
    }

    /// 从 XML 文档列表中提取启用的节点定义。
    ///
    /// 参数 `document_list` 对应 Java `documentList`，Rust 使用原始 XML 文本
    /// 代替 dom4j `Document`；`plan` 接收节点中间属性。返回 XML 解析结果。
    /// 对应 Java: `ParserHelper#parseNodeDocument`。
    pub fn parse_node_document(
        document_list: &[String],
        plan: &mut RuleDefinitionPlan,
    ) -> LFResult<()> {
        for document in document_list {
            let mut reader = xml_reader(document);
            let mut buffer = Vec::new();
            loop {
                match reader.read_event_into(&mut buffer) {
                    Ok(Event::Start(element)) if element.name().as_ref() == b"nodes" => {
                        read_nodes(&mut reader, plan)?;
                    }
                    Ok(Event::Start(element)) if element.name().as_ref() != b"flow" => {
                        let tag = String::from_utf8_lossy(element.name().as_ref()).to_string();
                        skip_element(&mut reader, &tag)?;
                    }
                    Ok(Event::Eof) => break,
                    Err(error) => return Err(rule_error(format!("xml parse error: {error}"))),
                    _ => {}
                }
                buffer.clear();
            }
        }
        Ok(())
    }

    /// 从 XML 文档列表中提取链定义。
    ///
    /// `document_list` 对应 Java `documentList`；`chain_id_set` 跨文档检测重复
    /// ID；`plan` 保存尚未编译的链定义。成功后清空检测集合，与 Java 行为一致。
    /// 对应 Java: `ParserHelper#parseChainDocument`。
    pub fn parse_chain_document(
        document_list: &[String],
        chain_id_set: &mut HashSet<String>,
        plan: &mut RuleDefinitionPlan,
    ) -> LFResult<()> {
        for document in document_list {
            let mut reader = xml_reader(document);
            let mut buffer = Vec::new();
            loop {
                match reader.read_event_into(&mut buffer) {
                    Ok(Event::Start(element)) if element.name().as_ref() == b"chain" => {
                        let (id, namespace, enabled, extends, executor_class) =
                            parse_chain_attrs(&element);
                        if enabled == Some(false) {
                            skip_element(&mut reader, "chain")?;
                            continue;
                        }
                        let id = id.ok_or_else(|| rule_error("missing chain id in expression"))?;
                        if !chain_id_set.insert(id.clone()) {
                            return Err(ChainDuplicateException::new(format!(
                                "[chain name duplicate] chainName={id}"
                            ))
                            .into());
                        }
                        let (route, body) = read_chain_content(&mut reader, &id)?;
                        let mut definition = ChainDef::new(id, body);
                        definition.namespace =
                            namespace.unwrap_or_else(|| DEFAULT_NAMESPACE.to_string());
                        definition.route = route;
                        definition.extends = extends;
                        definition.thread_pool_executor_class = executor_class;
                        plan.push_chain(definition);
                    }
                    Ok(Event::Empty(element)) if element.name().as_ref() == b"chain" => {
                        let (id, _, _, _, _) = parse_chain_attrs(&element);
                        let id = id.ok_or_else(|| rule_error("missing chain id in expression"))?;
                        return Err(rule_error(format!("chain[{id}] has empty EL")));
                    }
                    Ok(Event::Start(element))
                        if element.name().as_ref() != b"flow"
                            && element.name().as_ref() != b"chain" =>
                    {
                        let tag = String::from_utf8_lossy(element.name().as_ref()).to_string();
                        skip_element(&mut reader, &tag)?;
                    }
                    Ok(Event::Eof) => break,
                    Err(error) => return Err(rule_error(format!("xml parse error: {error}"))),
                    _ => {}
                }
                buffer.clear();
            }
        }
        chain_id_set.clear();
        Ok(())
    }
}

fn rule_error(message: impl Into<String>) -> LiteflowError {
    LiteflowError::Rule(message.into())
}

fn is_disabled(enable: Option<&Value>) -> bool {
    enable.is_some_and(|enable| {
        enable.as_bool() == Some(false)
            || enable
                .as_str()
                .is_some_and(|value| value.eq_ignore_ascii_case("false"))
    })
}

fn xml_reader(content: &str) -> Reader<&[u8]> {
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);
    reader
}

type ChainAttrs = (
    Option<String>,
    Option<String>,
    Option<bool>,
    Option<String>,
    Option<String>,
);

fn parse_chain_attrs(element: &BytesStart<'_>) -> ChainAttrs {
    let mut id = None;
    let mut namespace = None;
    let mut enabled = None;
    let mut extends = None;
    let mut executor_class = None;
    for attribute in element.attributes().flatten() {
        let key = String::from_utf8_lossy(attribute.key.as_ref());
        let value = String::from_utf8_lossy(&attribute.value).to_string();
        match key.as_ref() {
            "id" => id = Some(value),
            "name" if id.is_none() => id = Some(value),
            "namespace" => namespace = Some(value),
            "enable" => enabled = Some(!value.eq_ignore_ascii_case("false")),
            "extends" => extends = Some(value),
            "threadPoolExecutorClass" => executor_class = Some(value),
            _ => {}
        }
    }
    (id, namespace, enabled, extends, executor_class)
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

#[allow(clippy::too_many_arguments)]
fn build_chain_recursive(
    bus: &FlowBus,
    chain_id: &str,
    definitions: &HashMap<String, ChainDef>,
    nodes: &HashMap<String, NodePropBean>,
    resolved: &mut HashMap<String, String>,
    inheritance_path: &mut HashSet<String>,
    build_path: &mut HashSet<String>,
    built_nodes: &mut HashSet<String>,
) -> LFResult<()> {
    if bus.contains_chain(chain_id) {
        return Ok(());
    }
    if !build_path.insert(chain_id.to_string()) {
        return Err(LiteflowError::Parse(format!(
            "cyclic chain reference detected: {chain_id}"
        )));
    }

    let definition = definitions.get(chain_id).ok_or_else(|| {
        LiteflowError::ChainNotFound(format!("[chain not found] chainId={chain_id}"))
    })?;
    if definition.extends.is_none() && is_abstract_chain(&definition.body) {
        build_path.remove(chain_id);
        return Err(LiteflowError::ChainNotFound(format!(
            "[abstract chain cannot execute] chainId={chain_id}"
        )));
    }
    let body = resolve_body(chain_id, definitions, resolved, inheritance_path)?;

    // 先递归物化 EL 引用，确保 Chain 构建器能识别子链；脚本节点仅编译当前
    // 依赖闭包，保留 PARSE_ONE_ON_FIRST_EXEC 的延迟语义。
    let mut references = HashSet::new();
    collect_references(&parse_el(&body)?, &mut references);
    if let Some(route) = &definition.route {
        collect_references(&parse_el(route)?, &mut references);
    }
    for reference in references {
        if bus.contains_node(&reference) {
            continue;
        }
        if let Some(node) = nodes.get(&reference) {
            if built_nodes.insert(reference.clone()) {
                ParserHelper::build_node(bus, node.clone())?;
            }
        } else if definitions.contains_key(&reference) {
            build_chain_recursive(
                bus,
                &reference,
                definitions,
                nodes,
                resolved,
                inheritance_path,
                build_path,
                built_nodes,
            )?;
        }
    }

    let builder = LiteFlowChainELBuilder::new(bus.clone());
    let mut chain = match &definition.route {
        Some(route) => builder.build_route_chain(
            &definition.id,
            &definition.namespace,
            parse_el(route)?,
            parse_el(&body)?,
        )?,
        None => builder
            .build_chain(&definition.id, parse_el(&body)?)?
            .with_namespace(&definition.namespace),
    };
    if let Some(executor_class) = &definition.thread_pool_executor_class {
        chain.set_thread_pool_executor_class(executor_class);
    }
    bus.add_built_chain(chain);
    build_path.remove(chain_id);
    Ok(())
}

fn resolve_body(
    chain_id: &str,
    definitions: &HashMap<String, ChainDef>,
    resolved: &mut HashMap<String, String>,
    inheritance_path: &mut HashSet<String>,
) -> LFResult<String> {
    if let Some(body) = resolved.get(chain_id) {
        return Ok(body.clone());
    }
    if !inheritance_path.insert(chain_id.to_string()) {
        return Err(LiteflowError::Parse(format!(
            "cyclic chain inheritance detected: {chain_id}"
        )));
    }
    let definition = definitions.get(chain_id).ok_or_else(|| {
        LiteflowError::ChainNotFound(format!("[abstract chain not found] chainId={chain_id}"))
    })?;
    let body = match &definition.extends {
        Some(parent_id) => replace_abstract_chain(
            &resolve_body(parent_id, definitions, resolved, inheritance_path)?,
            &definition.body,
        )?,
        None => definition.body.clone(),
    };
    inheritance_path.remove(chain_id);
    resolved.insert(chain_id.to_string(), body.clone());
    Ok(body)
}

fn collect_references(el: &El, references: &mut HashSet<String>) {
    match el {
        El::Node(node) => {
            references.insert(node.id.clone());
        }
        El::Boolean(_) => {}
        El::Then(items) | El::And(items) | El::Or(items) => {
            for item in items {
                collect_references(item, references);
            }
        }
        El::When { items, .. } => {
            for item in items {
                collect_references(item, references);
            }
        }
        El::If {
            cond,
            then,
            elifs,
            els,
        } => {
            collect_references(cond, references);
            collect_references(then, references);
            for (condition, branch) in elifs {
                collect_references(condition, references);
                collect_references(branch, references);
            }
            if let Some(branch) = els {
                collect_references(branch, references);
            }
        }
        El::Switch {
            node,
            targets,
            default,
        } => {
            collect_references(node, references);
            for target in targets {
                collect_references(target, references);
            }
            if let Some(default) = default {
                collect_references(default, references);
            }
        }
        El::For {
            node, body, brk, ..
        }
        | El::While {
            node, body, brk, ..
        }
        | El::Iter {
            node, body, brk, ..
        } => {
            collect_references(node, references);
            collect_references(body, references);
            if let Some(brk) = brk {
                collect_references(brk, references);
            }
        }
        El::ForCount { body, brk, .. } => {
            collect_references(body, references);
            if let Some(brk) = brk {
                collect_references(brk, references);
            }
        }
        El::Catch { body, do_ } => {
            collect_references(body, references);
            if let Some(handler) = do_ {
                collect_references(handler, references);
            }
        }
        El::Not(item) | El::Pre(item) | El::Fin(item) | El::Mods(item, _) => {
            collect_references(item, references);
        }
    }
}
