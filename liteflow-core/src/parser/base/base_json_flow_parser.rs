//! JSON 规则解析器公共实现。

use crate::builder::NodePropBean;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::chain::DEFAULT_NAMESPACE;
use crate::flow::flow_bus::FlowBus;
use crate::parser::RuleDefinitionPlan;
use crate::parser::chain_def::ChainDef;
use serde_json::Value;

/// 承载 JSON/JSON-EL 解析器共享的节点与链路解析逻辑。
///
/// 多份规则文本先统一收集 `ChainDef`，再一次性处理继承关系，保证父链与子链
/// 分布在不同文件时仍可正确解析。
///
/// 对应 Java: `com.yomahub.liteflow.parser.base.BaseJsonFlowParser`。
#[derive(Clone)]
pub struct BaseJsonFlowParser {
    bus: FlowBus,
}

impl BaseJsonFlowParser {
    /// 使用目标流程总线创建解析器。
    ///
    /// 参数 `bus` 接收解析后节点和链路；返回可复用的解析器实例。
    #[must_use]
    pub fn new(bus: FlowBus) -> Self {
        Self { bus }
    }

    /// 解析 JSON 文本列表。
    ///
    /// 空列表与 Java 基类一致，视为无需加载并返回空结果。
    /// 对应 Java: `BaseJsonFlowParser#parse(List<String>)`。
    pub fn parse(&self, content_list: &[String]) -> LFResult<Vec<String>> {
        if content_list.is_empty() {
            return Ok(Vec::new());
        }

        self.collect(content_list)?.build_all(&self.bus)
    }

    /// 只解析格式并保存节点、链定义，不创建 Chain 或编译脚本。
    ///
    /// 对应 Java `PARSE_ONE_ON_FIRST_EXEC` 启动阶段预装载 Chain 定义的行为。
    pub fn collect(&self, content_list: &[String]) -> LFResult<RuleDefinitionPlan> {
        let mut values = Vec::with_capacity(content_list.len());
        for content in content_list {
            let value = serde_json::from_str(content)
                .map_err(|error| LiteflowError::Rule(format!("invalid json: {error}")))?;
            values.push(value);
        }
        self.collect_values(&values)
    }

    /// 收集已经转换为 JSON Value 的规则列表，供 YML 延迟解析复用。
    pub(crate) fn collect_values(&self, values: &[Value]) -> LFResult<RuleDefinitionPlan> {
        if values.is_empty() {
            return Ok(RuleDefinitionPlan::new());
        }

        let mut plan = RuleDefinitionPlan::new();
        for value in values {
            self.collect_value(value, &mut plan)?;
        }
        Ok(plan)
    }

    fn collect_value(&self, value: &Value, plan: &mut RuleDefinitionPlan) -> LFResult<()> {
        let flow = value
            .get("flow")
            .ok_or_else(|| LiteflowError::Rule("missing flow".to_string()))?;

        if let Some(nodes) = flow
            .get("nodes")
            .and_then(|nodes| nodes.get("node"))
            .and_then(Value::as_array)
        {
            for node in nodes {
                if let Some(node) = self.parse_node(node)? {
                    plan.push_node(node);
                }
            }
        }

        let chains = flow
            .get("chain")
            .and_then(Value::as_array)
            .ok_or_else(|| LiteflowError::Rule("missing flow.chain".to_string()))?;
        for chain in chains {
            plan.push_chain(parse_chain_definition(chain)?);
        }
        Ok(())
    }

    fn parse_node(&self, node: &Value) -> LFResult<Option<NodePropBean>> {
        if node
            .get("enable")
            .and_then(Value::as_bool)
            .is_some_and(|enable| !enable)
        {
            return Ok(None);
        }

        let id = node.get("id").and_then(Value::as_str).unwrap_or_default();
        let property: NodePropBean = serde_json::from_value(node.clone()).map_err(|error| {
            LiteflowError::Rule(format!("invalid node[{id}] property: {error}"))
        })?;
        Ok(Some(property))
    }
}

fn parse_chain_definition(chain: &Value) -> LFResult<ChainDef> {
    let id = chain
        .get("id")
        .or_else(|| chain.get("name"))
        .and_then(Value::as_str)
        .ok_or_else(|| LiteflowError::Rule("chain missing id/name".to_string()))?
        .to_string();
    let mut definition = ChainDef::new(id.clone(), "");
    definition.namespace = chain
        .get("namespace")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_NAMESPACE)
        .to_string();
    definition.enable = chain.get("enable").and_then(Value::as_bool).unwrap_or(true);
    definition.extends = chain
        .get("extends")
        .and_then(Value::as_str)
        .map(str::to_string);
    definition.route = chain
        .get("route")
        .and_then(Value::as_str)
        .map(str::to_string);

    if let Some(body) = chain.get("body").and_then(Value::as_str) {
        definition.body = body.to_string();
    } else if definition.route.is_some() {
        return Err(LiteflowError::Rule(format!(
            "If you have defined the field route, then you must define the field body in chain[{id}]"
        )));
    } else {
        let conditions = chain
            .get("condition")
            .and_then(Value::as_array)
            .ok_or_else(|| LiteflowError::Rule(format!("chain[{id}] missing condition")))?;
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
                .ok_or_else(|| {
                    LiteflowError::Rule(format!("chain[{id}] condition missing value"))
                })?;
            parts.push(format!("{condition_type}({value})"));
        }
        definition.body = if parts.len() == 1 {
            parts.remove(0)
        } else {
            format!("THEN({})", parts.join(","))
        };
    }
    Ok(definition)
}
