//! 链定义的中间表示与两阶段构建（对应 ParserHelper 的链解析 +
//! processChainInheritance 链继承解析）。

use crate::el::parse_el;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::chain::DEFAULT_NAMESPACE;
use crate::flow::flow_bus::FlowBus;
use crate::util::el_regex_util::{is_abstract_chain, replace_abstract_chain};

/// 规则文件中一条链的原始定义
#[derive(Debug, Clone)]
pub struct ChainDef {
    pub id: String,
    pub namespace: String,
    pub route: Option<String>,
    pub body: String,
    /// extends 属性（链继承）
    pub extends: Option<String>,
    /// enable=false 跳过
    pub enable: bool,
}

impl ChainDef {
    pub fn new(id: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            namespace: DEFAULT_NAMESPACE.to_string(),
            route: None,
            body: body.into(),
            extends: None,
            enable: true,
        }
    }
}

/// 两阶段构建：
/// 1. 解析链继承（父链含 {{占位符}} 的抽象 EL，子链提供实现）
/// 2. 统一构建（平滑加载：全部解析成功后原子写入）
pub fn resolve_and_build(bus: &FlowBus, defs: Vec<ChainDef>) -> LFResult<Vec<String>> {
    let defs: Vec<ChainDef> = defs.into_iter().filter(|d| d.enable).collect();
    let raw: std::collections::HashMap<String, ChainDef> =
        defs.iter().map(|d| (d.id.clone(), d.clone())).collect();

    let mut resolved: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut processed: std::collections::HashSet<String> = std::collections::HashSet::new();

    fn resolve(
        id: &str,
        raw: &std::collections::HashMap<String, ChainDef>,
        resolved: &mut std::collections::HashMap<String, String>,
        processed: &mut std::collections::HashSet<String>,
    ) -> LFResult<String> {
        if let Some(el) = resolved.get(id) {
            return Ok(el.clone());
        }
        if processed.contains(id) {
            return Err(LiteflowError::Parse(format!(
                "cyclic chain inheritance detected: {id}"
            )));
        }
        let def = raw.get(id).ok_or_else(|| {
            LiteflowError::ChainNotFound(format!("[abstract chain not found] chainId={id}"))
        })?;
        processed.insert(id.to_string());
        let el = match &def.extends {
            Some(parent_id) => {
                let parent_el = resolve(parent_id, raw, resolved, processed)?;
                replace_abstract_chain(&parent_el, &def.body)?
            }
            None => def.body.clone(),
        };
        resolved.insert(id.to_string(), el.clone());
        Ok(el)
    }

    let mut ids = Vec::new();
    // 抽象链（含占位符）不注册为可执行链（对齐 Java：abstract chain 不可直接执行）
    for def in &defs {
        if def.extends.is_none() && is_abstract_chain(&def.body) {
            resolved.insert(def.id.clone(), def.body.clone());
            continue;
        }
        let body_el = resolve(&def.id, &raw, &mut resolved, &mut processed)?;
        match &def.route {
            Some(r) => bus.add_route_chain(def.id.clone(), &def.namespace, r, &body_el)?,
            None => {
                let el = parse_el(&body_el)?;
                let chain =
                    crate::builder::el::lite_flow_chain_el_builder::LiteFlowChainELBuilder::new(
                        bus.clone(),
                    )
                    .build_chain(&def.id, el)?
                    .with_namespace(&def.namespace);
                bus.add_built_chain(chain);
            }
        }
        ids.push(def.id.clone());
    }
    Ok(ids)
}
