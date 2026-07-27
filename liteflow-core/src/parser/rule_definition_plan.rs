//! 规则定义的延迟物化计划。

use std::collections::{HashMap, HashSet};

use crate::builder::NodePropBean;
use crate::builder::el::lite_flow_chain_el_builder::LiteFlowChainELBuilder;
use crate::el::{El, parse_el};
use crate::exception::{LFResult, LiteflowError};
use crate::flow::flow_bus::FlowBus;
use crate::parser::chain_def::ChainDef;
use crate::parser::helper::ParserHelper;
use crate::util::el_regex_util::{is_abstract_chain, replace_abstract_chain};

/// 保存规则文件中的节点与链定义，并按解析模式选择一次性或按链物化。
///
/// Java `PARSE_ONE_ON_FIRST_EXEC` 会在启动期保存 Chain 定义，在链第一次执行时
/// 才构建相关 Chain 并编译其脚本节点；该对象是 Rust 端等价的中间状态载体。
/// 对应 Java: `com.yomahub.liteflow.parser.helper.ParserHelper` 的延迟解析职责。
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
    pub fn push_node(&mut self, node: NodePropBean) {
        self.nodes.push(node);
    }

    /// 追加一个已通过格式解析的链定义。
    pub fn push_chain(&mut self, chain: ChainDef) {
        self.chains.push(chain);
    }

    /// 返回计划中可执行或抽象链定义的数量。
    #[must_use]
    pub fn chain_count(&self) -> usize {
        self.chains.iter().filter(|chain| chain.enable).count()
    }

    /// 立即构建全部节点与全部可执行链。
    ///
    /// 参数 `bus` 接收真实节点、脚本执行器和 Chain；返回成功注册的 chain id。
    /// 对应 Java: `PARSE_ALL_ON_START` 与 `PARSE_ALL_ON_FIRST_EXEC` 的全量解析阶段。
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
        // 逐个入口按依赖拓扑构建；声明顺序不应决定子链能否被识别。
        for chain_id in &chain_ids {
            self.build_chain(bus, chain_id)?;
        }
        Ok(chain_ids)
    }

    /// 仅构建指定链以及它递归引用的子链和节点。
    ///
    /// 未被该链引用的脚本节点不会编译；并发调用者可由上层初始化锁串行化。
    /// 参数 `chain_id` 对应 Java `FlowExecutor#doExecute` 的执行链标识。
    /// 对应 Java: `PARSE_ONE_ON_FIRST_EXEC`。
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
        let mut resolved = HashMap::new();
        let mut inheritance_path = HashSet::new();
        let mut build_path = HashSet::new();
        let mut built_nodes = HashSet::new();

        build_chain_recursive(
            bus,
            chain_id,
            &definitions,
            &nodes,
            &mut resolved,
            &mut inheritance_path,
            &mut build_path,
            &mut built_nodes,
        )
    }
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

    // 先递归物化 EL 引用，确保 Chain 构建器能把子链识别为 ChainBindWrapper，
    // 同时只编译当前依赖闭包内的脚本节点。
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

    match &definition.route {
        Some(route) => {
            bus.add_route_chain(definition.id.clone(), &definition.namespace, route, &body)?
        }
        None => {
            let chain = LiteFlowChainELBuilder::new(bus.clone())
                .build_chain(&definition.id, parse_el(&body)?)?
                .with_namespace(&definition.namespace);
            bus.add_built_chain(chain);
        }
    }
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
