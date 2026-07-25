//! 对应 LiteFlowChainELBuilder：把 EL 语法树构建为 Condition 对象树。
//!
//! Java 版在构建期解析节点 class、实例化组件并校验；Rust 版在构建期
//! 从 FlowBus 解析组件实例（NodeBuildException 语义），并一次性组装
//! Chain（平滑加载：构建完成前不影响在跑的旧链路）。

use crate::el::{El, NodeRef};
use crate::enums::BooleanConditionTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::chain::Chain;
use crate::flow::element::condition::{
    and_or_condition::AndOrCondition, catch_condition::CatchCondition,
    finally_condition::FinallyCondition, for_condition::ForCondition,
    if_condition::IfCondition, ignore_error_condition::IgnoreErrorCondition,
    iterator_condition::IteratorCondition, not_condition::NotCondition,
    pre_condition::PreCondition, retry_condition::RetryCondition,
    switch_condition::SwitchCondition, then_condition::ThenCondition,
    timeout_condition::TimeoutCondition, when_condition::WhenCondition,
    while_condition::WhileCondition,
};
use crate::core::decl_component::DeclMethodComponent;
use crate::flow::element::condition::chain_bind_wrapper_condition::ChainBindWrapperCondition;
use crate::flow::element::node::{Node, NodeHooks};
use crate::flow::element::Executable;
use crate::flow::flow_bus::FlowBus;
use std::collections::HashMap;
use std::sync::Arc;

pub struct LiteFlowChainELBuilder {
    bus: FlowBus,
    chain_id: std::cell::RefCell<String>,
    /// 节点出现次数计数（NodeInstanceIdManageSpi 语义）
    occurrences: std::cell::RefCell<HashMap<String, usize>>,
}

impl LiteFlowChainELBuilder {
    pub fn new(bus: FlowBus) -> Self {
        Self {
            bus,
            chain_id: std::cell::RefCell::new(String::new()),
            occurrences: std::cell::RefCell::new(HashMap::new()),
        }
    }

    /// buildCondition → 组装 Chain
    pub fn build_chain(&self, chain_id: &str, el: El) -> LFResult<Chain> {
        *self.chain_id.borrow_mut() = chain_id.to_string();
        let cond = self.build_executable(el)?;
        Ok(Chain::new(chain_id, vec![cond]))
    }

    /// setRoute + setEL：构建决策表链路（对应 routeItem）
    pub fn build_route_chain(
        &self,
        chain_id: &str,
        namespace: &str,
        route_el: El,
        body_el: El,
    ) -> LFResult<Chain> {
        *self.chain_id.borrow_mut() = chain_id.to_string();
        let route = self.build_executable(route_el)?;
        let body = self.build_executable(body_el)?;
        let mut chain = Chain::new(chain_id, vec![body]).with_namespace(namespace);
        chain.set_route_item(route);
        Ok(chain)
    }

    fn build_executable(&self, el: El) -> LFResult<Arc<dyn Executable>> {
        match el {
            El::Node(n) => self.build_node_or_chain(n),
            El::Then(items) => {
                let mut cond = ThenCondition::new();
                for item in items {
                    match item {
                        El::Pre(inner) => {
                            cond.add_pre_condition(Arc::new(PreCondition::new(
                                self.build_executable(*inner)?,
                            )));
                        }
                        El::Fin(inner) => {
                            cond.add_finally_condition(Arc::new(FinallyCondition::new(
                                self.build_executable(*inner)?,
                            )));
                        }
                        other => cond.add_executable(self.build_executable(other)?),
                    }
                }
                Ok(Arc::new(cond))
            }
            El::When { items, opts } => {
                let list = items
                    .into_iter()
                    .map(|i| self.build_executable(i))
                    .collect::<LFResult<Vec<_>>>()?;
                let mut cond = WhenCondition::new(list);
                cond.ignore_error = opts.ignore_error;
                cond.any = opts.any;
                cond.percentage = opts.percentage;
                cond.must = opts.must;
                cond.max_wait_ms = opts.max_wait_ms;
                cond.thread_executor = opts.thread_pool;
                Ok(Arc::new(cond))
            }
            El::If { cond, then, elifs, els } => {
                let if_item = self.build_executable(*cond)?;
                let true_case = self.build_executable(*then)?;
                let elif_list = elifs
                    .into_iter()
                    .map(|(c, t)| Ok((self.build_executable(c)?, self.build_executable(t)?)))
                    .collect::<LFResult<Vec<_>>>()?;
                let false_case = els.map(|e| self.build_executable(*e)).transpose()?;
                Ok(Arc::new(IfCondition::new(if_item, true_case, elif_list, false_case)))
            }
            El::Switch { node, targets, default } => {
                let switch_node = self.build_executable(*node)?;
                let target_list = targets
                    .into_iter()
                    .map(|t| self.build_executable(t))
                    .collect::<LFResult<Vec<_>>>()?;
                let default_executor = default.map(|d| self.build_executable(*d)).transpose()?;
                Ok(Arc::new(SwitchCondition::new(switch_node, target_list, default_executor)))
            }
            El::For { node, parallel, body, brk } => {
                Ok(Arc::new(ForCondition::new(
                    self.build_executable(*node)?,
                    parallel,
                    self.build_executable(*body)?,
                    brk.map(|b| self.build_executable(*b)).transpose()?,
                )))
            }
            El::While { node, parallel, body, brk } => {
                Ok(Arc::new(WhileCondition::new(
                    self.build_executable(*node)?,
                    parallel,
                    self.build_executable(*body)?,
                    brk.map(|b| self.build_executable(*b)).transpose()?,
                )))
            }
            El::Iter { node, parallel, body, brk } => {
                Ok(Arc::new(IteratorCondition::new(
                    self.build_executable(*node)?,
                    parallel,
                    self.build_executable(*body)?,
                    brk.map(|b| self.build_executable(*b)).transpose()?,
                )))
            }
            El::Catch { body, do_ } => {
                Ok(Arc::new(CatchCondition::new(
                    self.build_executable(*body)?,
                    do_.map(|d| self.build_executable(*d)).transpose()?,
                )))
            }
            El::And(items) => Ok(Arc::new(AndOrCondition::new(
                BooleanConditionTypeEnum::And,
                items.into_iter().map(|i| self.build_executable(i)).collect::<LFResult<_>>()?,
            ))),
            El::Or(items) => Ok(Arc::new(AndOrCondition::new(
                BooleanConditionTypeEnum::Or,
                items.into_iter().map(|i| self.build_executable(i)).collect::<LFResult<_>>()?,
            ))),
            El::Not(item) => Ok(Arc::new(NotCondition::new(self.build_executable(*item)?))),
            El::Pre(inner) => Ok(Arc::new(PreCondition::new(self.build_executable(*inner)?))),
            El::Fin(inner) => Ok(Arc::new(FinallyCondition::new(self.build_executable(*inner)?))),
            El::Mods(inner, mods) => {
                let mut target = self.build_executable(*inner)?;
                if let Some(r) = mods.retry {
                    target = Arc::new(RetryCondition::new(target, r));
                }
                if let Some(ms) = mods.max_wait_ms {
                    target = Arc::new(TimeoutCondition::new(target, ms));
                }
                if mods.ignore_error {
                    target = Arc::new(IgnoreErrorCondition::new(target));
                }
                Ok(target)
            }
        }
    }

    /// 对应 ComponentInitializer 的实例解析：
    /// 1. 普通组件 2. 声明式组件方法（cmpId.method）3. 子链（ChainBindWrapperCondition）
    fn build_node(&self, node_ref: NodeRef) -> LFResult<Node> {
        // 声明式组件方法引用：cmpId.methodName
        if let Some((cmp_id, method)) = node_ref.id.split_once('.') {
            let decl = self.bus.get_decl(cmp_id).ok_or_else(|| {
                LiteflowError::NodeBuild(format!("decl component[{cmp_id}] not registered"))
            })?;
            let instance: Arc<dyn crate::core::node_component::NodeComponent> =
                Arc::new(DeclMethodComponent::new(decl, method));
            return Ok(self.finish_node(node_ref, instance));
        }
        let instance = self
            .bus
            .get_node(&node_ref.id)
            .ok_or_else(|| LiteflowError::NodeBuild(format!(
                "node[{}] not registered",
                node_ref.id
            )))?;
        Ok(self.finish_node(node_ref, instance))
    }

    /// 注入实例编号与横切钩子，并触发 node_build 生命周期
    fn finish_node(
        &self,
        node_ref: NodeRef,
        instance: Arc<dyn crate::core::node_component::NodeComponent>,
    ) -> Node {
        let chain_id = self.chain_id.borrow().clone();
        let mut occ = self.occurrences.borrow_mut();
        let occurrence = {
            let e = occ.entry(node_ref.id.clone()).or_insert(0);
            let v = *e;
            *e += 1;
            v
        };
        let instance_id = self
            .bus
            .instance_id_spi
            .read()
            .unwrap()
            .gen_instance_id(&chain_id, &node_ref.id, occurrence);
        let (aspects, monitor) = self.bus.hooks_snapshot();
        let hooks = NodeHooks {
            aspects,
            monitor: Some(monitor),
        };
        for h in &self.bus.lifecycle.read().unwrap().node_build {
            h.post_process_after_node_build(&node_ref.id);
        }
        Node::new(node_ref, instance)
            .with_instance_id(instance_id)
            .with_hooks(hooks)
    }

    /// 子链包装（对应 ChainBindWrapperCondition 的构建时机）
    fn build_node_or_chain(&self, node_ref: NodeRef) -> LFResult<Arc<dyn Executable>> {
        let id_no_method = node_ref.id.split('.').next().unwrap_or("");
        if self.bus.contains_node(&node_ref.id) || self.bus.get_decl(id_no_method).is_some() {
            return Ok(Arc::new(self.build_node(node_ref)?));
        }
        if let Some(chain) = self.bus.get_chain(&node_ref.id) {
            return Ok(Arc::new(ChainBindWrapperCondition::new(chain)));
        }
        Err(LiteflowError::NodeBuild(format!(
            "node[{}] not registered",
            node_ref.id
        )))
    }
}
