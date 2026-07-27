//! 把 EL 语法树构建为可执行的 Condition 对象树。
//!
//! Java 版在构建期解析节点 class、实例化组件并校验；Rust 版在构建期
//! 从 FlowBus 解析组件实例（NodeBuildException 语义），并一次性组装
//! Chain（平滑加载：构建完成前不影响在跑的旧链路）。
//!
//! 对应 Java: `com.yomahub.liteflow.builder.el.LiteFlowChainELBuilder`。

use crate::builder::el::operator::boolean_literal_condition::BooleanLiteralCondition;
use crate::common::entity::ValidationResp;
use crate::core::DeclMethodComponent;
use crate::el::{El, NodeRef, format_el_parse_error, parse_el};
use crate::enums::NodeTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::Executable;
use crate::flow::element::NodeHooks;
use crate::flow::element::chain::Chain;
use crate::flow::element::condition::BooleanConditionTypeEnum;
use crate::flow::element::condition::bind_wrapper_condition::BindWrapperCondition;
use crate::flow::element::condition::chain_bind_wrapper_condition::ChainBindWrapperCondition;
use crate::flow::element::condition::{
    and_or_condition::AndOrCondition, catch_condition::CatchCondition,
    finally_condition::FinallyCondition, for_condition::ForCondition, if_condition::IfCondition,
    ignore_error_condition::IgnoreErrorCondition, iterator_condition::IteratorCondition,
    not_condition::NotCondition, pre_condition::PreCondition, retry_condition::RetryCondition,
    switch_condition::SwitchCondition, then_condition::ThenCondition,
    timeout_condition::TimeoutCondition, when_condition::WhenCondition,
    while_condition::WhileCondition,
};
use crate::flow::element::fallback_node::FallbackNode;
use crate::flow::element::node::Node;
use crate::flow::flow_bus::FlowBus;
use std::collections::HashMap;
use std::sync::Arc;

/// LiteFlow 链路 EL 构建器。
///
/// 保存当前链路标识和节点出现次数，将解析后的 EL 递归转换为 Chain、
/// Condition、Node 与 FallbackNode。对应 Java:
/// `com.yomahub.liteflow.builder.el.LiteFlowChainELBuilder`。
pub struct LiteFlowChainELBuilder {
    bus: FlowBus,
    chain_id: std::cell::RefCell<String>,
    /// 节点出现次数计数（NodeInstanceIdManageSpi 语义）
    occurrences: std::cell::RefCell<HashMap<String, usize>>,
}

impl LiteFlowChainELBuilder {
    /// 创建绑定指定 FlowBus 的构建器。
    ///
    /// Java 构造器接收 Chain；Rust 构建器接收组件和链路注册中心，以便解析节点
    /// 实例。对应 Java: `LiteFlowChainELBuilder#LiteFlowChainELBuilder`。
    pub fn new(bus: FlowBus) -> Self {
        Self {
            bus,
            chain_id: std::cell::RefCell::new(String::new()),
            occurrences: std::cell::RefCell::new(HashMap::new()),
        }
    }

    /// 校验 EL 表达式是否合法。
    ///
    /// 解析语法后还会依据当前构建器绑定的 `FlowBus` 校验节点、声明式组件和子链
    /// 是否已经注册。对应 Java: `LiteFlowChainELBuilder#validate`。
    #[must_use]
    pub fn validate(&self, el_str: &str) -> bool {
        self.validate_with_ex(el_str).is_success()
    }

    /// 校验 EL 表达式并保留精细化失败原因。
    ///
    /// 失败信息包含原始 EL、Unicode 安全的行列号和 `^` 错误位置；未注册引用使用
    /// Java `buildDataNotFoundExceptionMsg` 的说明语义。参数 `el_str` 是待校验 EL，
    /// 返回值包含成功标记及 `ELParseException` 对应错误。
    /// 对应 Java: `LiteFlowChainELBuilder#validateWithEx`。
    #[must_use]
    pub fn validate_with_ex(&self, el_str: &str) -> ValidationResp {
        let expression = match parse_el(el_str) {
            Ok(expression) => expression,
            Err(error) => return ValidationResp::fail(error),
        };
        if let Some(node) = first_unregistered_node(&expression, &self.bus) {
            let position = find_node_position(el_str, &node.id).unwrap_or(0);
            return ValidationResp::fail(format_el_parse_error(
                el_str,
                position,
                format!(
                    "[{}] is not exist or [{}] is not registered, you need to define a node or chain with id [{}] and register it",
                    node.id, node.id, node.id
                ),
            ));
        }
        ValidationResp::success()
    }

    /// 根据 EL 语法树构建普通 Chain。
    ///
    /// 参数 `chain_id` 对应 Java Chain 的 chainId，`el` 对应两阶段解析后的
    /// condition 表达式。返回完整构建且尚未注册的 Chain。
    /// 对应 Java: `LiteFlowChainELBuilder#setEL`。
    pub fn build_chain(&self, chain_id: &str, el: El) -> LFResult<Chain> {
        *self.chain_id.borrow_mut() = chain_id.to_string();
        let cond = self.build_executable(el)?;
        Ok(Chain::new(chain_id, vec![cond]))
    }

    /// 构建包含 route 条件和 body 的决策表链路。
    ///
    /// `namespace` 用于路由链分组，`route_el` 决定是否命中，`body_el` 是命中后
    /// 的执行体。对应 Java: `LiteFlowChainELBuilder#setRoute` 与 `#setEL`。
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
        self.build_executable_as(el, NodeTypeEnum::Common)
    }

    /// 递归构建可执行对象，并把当前位置要求的节点类型传给缺失节点代理。
    ///
    /// 对应 Java `FallbackNode#findFallbackNode` 对父 Condition 与节点位置的
    /// 运行期判断；Rust 在构建期已经拥有完整 AST，因此直接静态推导。
    fn build_executable_as(
        &self,
        el: El,
        expected_node_type: NodeTypeEnum,
    ) -> LFResult<Arc<dyn Executable>> {
        match el {
            El::Node(n) => self.build_node_or_chain(n, expected_node_type),
            El::Boolean(value) => Ok(Arc::new(BooleanLiteralCondition::new(value))),
            El::Then(items) => {
                let mut cond = ThenCondition::new();
                for item in items {
                    match item {
                        El::Pre(inner) => {
                            cond.add_pre_condition(Arc::new(PreCondition::new(
                                self.build_executable_as(*inner, NodeTypeEnum::Common)?,
                            )));
                        }
                        El::Fin(inner) => {
                            cond.add_finally_condition(Arc::new(FinallyCondition::new(
                                self.build_executable_as(*inner, NodeTypeEnum::Common)?,
                            )));
                        }
                        other => cond
                            .add_executable(self.build_executable_as(other, NodeTypeEnum::Common)?),
                    }
                }
                Ok(Arc::new(cond))
            }
            El::When { items, opts } => {
                let list = items
                    .into_iter()
                    .map(|i| self.build_executable_as(i, NodeTypeEnum::Common))
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
            El::If {
                cond,
                then,
                elifs,
                els,
            } => {
                let if_item = self.build_executable_as(*cond, NodeTypeEnum::Boolean)?;
                let true_case = self.build_executable_as(*then, NodeTypeEnum::Common)?;
                let elif_list = elifs
                    .into_iter()
                    .map(|(c, t)| {
                        Ok((
                            self.build_executable_as(c, NodeTypeEnum::Boolean)?,
                            self.build_executable_as(t, NodeTypeEnum::Common)?,
                        ))
                    })
                    .collect::<LFResult<Vec<_>>>()?;
                let false_case = els
                    .map(|e| self.build_executable_as(*e, NodeTypeEnum::Common))
                    .transpose()?;
                Ok(Arc::new(IfCondition::new(
                    if_item, true_case, elif_list, false_case,
                )))
            }
            El::Switch {
                node,
                targets,
                default,
            } => {
                let switch_node = self.build_executable_as(*node, NodeTypeEnum::Switch)?;
                let target_list = targets
                    .into_iter()
                    .map(|t| self.build_executable_as(t, NodeTypeEnum::Common))
                    .collect::<LFResult<Vec<_>>>()?;
                let default_executor = default
                    .map(|d| self.build_executable_as(*d, NodeTypeEnum::Common))
                    .transpose()?;
                Ok(Arc::new(SwitchCondition::new(
                    switch_node,
                    target_list,
                    default_executor,
                )))
            }
            El::For {
                node,
                parallel,
                body,
                brk,
            } => Ok(Arc::new(ForCondition::new(
                self.build_executable_as(*node, NodeTypeEnum::For)?,
                parallel,
                self.build_executable_as(*body, NodeTypeEnum::Common)?,
                brk.map(|b| self.build_executable_as(*b, NodeTypeEnum::Boolean))
                    .transpose()?,
            ))),
            El::ForCount {
                count,
                parallel,
                body,
                brk,
            } => Ok(Arc::new(ForCondition::with_count(
                count,
                parallel,
                self.build_executable_as(*body, NodeTypeEnum::Common)?,
                brk.map(|b| self.build_executable_as(*b, NodeTypeEnum::Boolean))
                    .transpose()?,
            ))),
            El::While {
                node,
                parallel,
                body,
                brk,
            } => Ok(Arc::new(WhileCondition::new(
                self.build_executable_as(*node, NodeTypeEnum::Boolean)?,
                parallel,
                self.build_executable_as(*body, NodeTypeEnum::Common)?,
                brk.map(|b| self.build_executable_as(*b, NodeTypeEnum::Boolean))
                    .transpose()?,
            ))),
            El::Iter {
                node,
                parallel,
                body,
                brk,
            } => Ok(Arc::new(IteratorCondition::new(
                self.build_executable_as(*node, NodeTypeEnum::Iterator)?,
                parallel,
                self.build_executable_as(*body, NodeTypeEnum::Common)?,
                brk.map(|b| self.build_executable_as(*b, NodeTypeEnum::Boolean))
                    .transpose()?,
            ))),
            El::Catch { body, do_ } => Ok(Arc::new(CatchCondition::new(
                self.build_executable_as(*body, NodeTypeEnum::Common)?,
                do_.map(|d| self.build_executable_as(*d, NodeTypeEnum::Common))
                    .transpose()?,
            ))),
            El::And(items) => Ok(Arc::new(AndOrCondition::new(
                BooleanConditionTypeEnum::And,
                items
                    .into_iter()
                    .map(|i| self.build_executable_as(i, NodeTypeEnum::Boolean))
                    .collect::<LFResult<_>>()?,
            ))),
            El::Or(items) => Ok(Arc::new(AndOrCondition::new(
                BooleanConditionTypeEnum::Or,
                items
                    .into_iter()
                    .map(|i| self.build_executable_as(i, NodeTypeEnum::Boolean))
                    .collect::<LFResult<_>>()?,
            ))),
            El::Not(item) => Ok(Arc::new(NotCondition::new(
                self.build_executable_as(*item, NodeTypeEnum::Boolean)?,
            ))),
            El::Pre(inner) => Ok(Arc::new(PreCondition::new(
                self.build_executable_as(*inner, NodeTypeEnum::Common)?,
            ))),
            El::Fin(inner) => Ok(Arc::new(FinallyCondition::new(
                self.build_executable_as(*inner, NodeTypeEnum::Common)?,
            ))),
            El::Mods(inner, mods) => {
                // 场景2：对 Condition bind（2.14+），override=true 时
                // 清除该 Condition 下所有 Node 上相同 key 的 bind（对齐 BindOperator）
                let mut inner_el = *inner;
                if mods.bind_override && !mods.bind.is_empty() {
                    for (k, _) in &mods.bind {
                        clear_node_bind(&mut inner_el, k);
                    }
                }
                let mut target = self.build_executable_as(inner_el, expected_node_type)?;
                if let Some(r) = mods.retry {
                    target = if mods.retry_for.is_empty() {
                        Arc::new(RetryCondition::new(target, r))
                    } else {
                        Arc::new(RetryCondition::with_exceptions(target, r, mods.retry_for))
                    };
                }
                if let Some(ms) = mods.max_wait_ms {
                    target = Arc::new(TimeoutCondition::new(target, ms));
                }
                if mods.ignore_error {
                    target = Arc::new(IgnoreErrorCondition::new(target));
                }
                if !mods.bind.is_empty()
                    || mods.id.is_some()
                    || mods.tag.is_some()
                    || mods.thread_pool.is_some()
                {
                    target = Arc::new(BindWrapperCondition::with_properties(
                        target,
                        mods.bind,
                        mods.id,
                        mods.tag,
                        mods.thread_pool,
                    ));
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
            if !decl.has_method(method) {
                return Err(LiteflowError::NodeBuild(format!(
                    "decl component[{cmp_id}] method[{method}] not registered"
                )));
            }
            let instance: Arc<dyn crate::core::node_component::NodeComponent> =
                Arc::new(DeclMethodComponent::new(decl, method));
            return Ok(self.finish_node(node_ref, instance));
        }
        let instance = self.bus.get_node(&node_ref.id).ok_or_else(|| {
            LiteflowError::NodeBuild(format!("node[{}] not registered", node_ref.id))
        })?;
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
        let instance_info = self
            .bus
            .instance_id_spi_holder
            .get_node_instance_id_manage_spi()
            .build_instance_info(&chain_id, &node_ref.id, occurrence);
        let instance_id = instance_info
            .instance_id()
            .expect("实例编号 SPI 必须生成 instanceId")
            .to_string();
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
    fn build_node_or_chain(
        &self,
        node_ref: NodeRef,
        expected_node_type: NodeTypeEnum,
    ) -> LFResult<Arc<dyn Executable>> {
        let id_no_method = node_ref.id.split('.').next().unwrap_or("");
        if self.bus.contains_node(&node_ref.id) || self.bus.get_decl(id_no_method).is_some() {
            return Ok(Arc::new(self.build_node(node_ref)?));
        }
        if let Some(chain) = self.bus.get_chain(&node_ref.id) {
            // 场景3：对 Chain bind（2.16）：包装成 ChainBindWrapperCondition 持有 bind 数据
            let mut wrapper = ChainBindWrapperCondition::new(chain);
            for (k, v) in node_ref.bind {
                wrapper.put_bind_data(k, v);
            }
            if let Some(tag) = node_ref.tag {
                wrapper.set_tag(tag);
            }
            return Ok(Arc::new(wrapper));
        }
        // 对应 Java NodeOperator：节点不存在时不在构建期失败，而是创建
        // FallbackNode，执行期先重查原节点，再按当前位置类型选择降级组件。
        let proxy: Arc<dyn crate::core::node_component::NodeComponent> =
            Arc::new(FallbackNode::new(
                node_ref.id.clone(),
                expected_node_type,
                self.bus.nodes.clone(),
                self.bus.fallback_nodes.clone(),
            ));
        Ok(Arc::new(self.finish_node(node_ref, proxy)))
    }
}

/// 深度优先查找第一个未注册节点或子链，顺序与 EL 源码的执行顺序一致。
fn first_unregistered_node<'a>(el: &'a El, bus: &FlowBus) -> Option<&'a NodeRef> {
    match el {
        El::Node(node) => {
            let declaration_id = node.id.split('.').next().unwrap_or("");
            (!bus.contains_node(&node.id)
                && !bus.contains_chain(&node.id)
                && bus.get_decl(declaration_id).is_none())
            .then_some(node)
        }
        El::Boolean(_) => None,
        El::Then(items) | El::And(items) | El::Or(items) | El::When { items, .. } => items
            .iter()
            .find_map(|item| first_unregistered_node(item, bus)),
        El::If {
            cond,
            then,
            elifs,
            els,
        } => first_unregistered_node(cond, bus)
            .or_else(|| first_unregistered_node(then, bus))
            .or_else(|| {
                elifs.iter().find_map(|(condition, body)| {
                    first_unregistered_node(condition, bus)
                        .or_else(|| first_unregistered_node(body, bus))
                })
            })
            .or_else(|| {
                els.as_deref()
                    .and_then(|item| first_unregistered_node(item, bus))
            }),
        El::Switch {
            node,
            targets,
            default,
        } => first_unregistered_node(node, bus)
            .or_else(|| {
                targets
                    .iter()
                    .find_map(|target| first_unregistered_node(target, bus))
            })
            .or_else(|| {
                default
                    .as_deref()
                    .and_then(|item| first_unregistered_node(item, bus))
            }),
        El::For {
            node, body, brk, ..
        }
        | El::While {
            node, body, brk, ..
        }
        | El::Iter {
            node, body, brk, ..
        } => first_unregistered_node(node, bus)
            .or_else(|| first_unregistered_node(body, bus))
            .or_else(|| {
                brk.as_deref()
                    .and_then(|item| first_unregistered_node(item, bus))
            }),
        El::ForCount { body, brk, .. } => first_unregistered_node(body, bus).or_else(|| {
            brk.as_deref()
                .and_then(|item| first_unregistered_node(item, bus))
        }),
        El::Catch { body, do_ } => first_unregistered_node(body, bus).or_else(|| {
            do_.as_deref()
                .and_then(|item| first_unregistered_node(item, bus))
        }),
        El::Not(item) | El::Pre(item) | El::Fin(item) | El::Mods(item, _) => {
            first_unregistered_node(item, bus)
        }
    }
}

/// 返回完整节点标识第一次作为独立标识符出现的 Unicode 字符偏移。
fn find_node_position(source: &str, node_id: &str) -> Option<usize> {
    let source_characters: Vec<char> = source.chars().collect();
    let node_characters: Vec<char> = node_id.chars().collect();
    if node_characters.is_empty() || node_characters.len() > source_characters.len() {
        return None;
    }
    source_characters
        .windows(node_characters.len())
        .enumerate()
        .find_map(|(index, window)| {
            if window != node_characters.as_slice() {
                return None;
            }
            let is_identifier_character = |character: char| {
                character.is_alphanumeric()
                    || character == '_'
                    || character == '$'
                    || character == '.'
            };
            let left_is_boundary =
                index == 0 || !is_identifier_character(source_characters[index.saturating_sub(1)]);
            let right_index = index + node_characters.len();
            let right_is_boundary = right_index == source_characters.len()
                || !is_identifier_character(source_characters[right_index]);
            (left_is_boundary && right_is_boundary).then_some(index)
        })
}

/// 递归清除语法树中所有 Node 上指定 key 的 bind（对应 BindOperator.clearNodeBindData）
fn clear_node_bind(el: &mut El, key: &str) {
    match el {
        El::Node(n) => n.bind.retain(|(k, _)| k != key),
        El::Boolean(_) => {}
        El::Then(items) | El::And(items) | El::Or(items) => {
            items.iter_mut().for_each(|i| clear_node_bind(i, key))
        }
        El::When { items, .. } => items.iter_mut().for_each(|i| clear_node_bind(i, key)),
        El::If {
            cond,
            then,
            elifs,
            els,
        } => {
            clear_node_bind(cond, key);
            clear_node_bind(then, key);
            elifs.iter_mut().for_each(|(c, t)| {
                clear_node_bind(c, key);
                clear_node_bind(t, key);
            });
            if let Some(e) = els {
                clear_node_bind(e, key);
            }
        }
        El::Switch {
            node,
            targets,
            default,
        } => {
            clear_node_bind(node, key);
            targets.iter_mut().for_each(|t| clear_node_bind(t, key));
            if let Some(d) = default {
                clear_node_bind(d, key);
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
            clear_node_bind(node, key);
            clear_node_bind(body, key);
            if let Some(b) = brk {
                clear_node_bind(b, key);
            }
        }
        El::ForCount { body, brk, .. } => {
            clear_node_bind(body, key);
            if let Some(b) = brk {
                clear_node_bind(b, key);
            }
        }
        El::Catch { body, do_ } => {
            clear_node_bind(body, key);
            if let Some(d) = do_ {
                clear_node_bind(d, key);
            }
        }
        El::Not(inner) | El::Pre(inner) | El::Fin(inner) => clear_node_bind(inner, key),
        El::Mods(inner, _) => clear_node_bind(inner, key),
    }
}
