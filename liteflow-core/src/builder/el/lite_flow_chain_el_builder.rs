//! 把 EL 语法树构建为可执行的 Condition 对象树。
//!
//! Java 版在构建期解析节点 class、实例化组件并校验；Rust 版在构建期
//! 从 FlowBus 解析组件实例（NodeBuildException 语义），并一次性组装
//! Chain（平滑加载：构建完成前不影响在跑的旧链路）。
//!
//! 对应 Java: `com.yomahub.liteflow.builder.el.LiteFlowChainELBuilder`。

use crate::builder::el::operator::base::OperatorHelper;
use crate::builder::el::operator::boolean_literal_condition::BooleanLiteralCondition;
use crate::common::ChainConstant;
use crate::common::entity::ValidationResp;
use crate::core::DeclMethodComponent;
use crate::el::{El, NodeRef, format_el_parse_error, parse_el};
use crate::enums::{NodeTypeEnum, ParseModeEnum};
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::Executable;
use crate::flow::element::NodeHooks;
use crate::flow::element::chain::Chain;
use crate::flow::element::condition::BooleanConditionTypeEnum;
use crate::flow::element::condition::bind_wrapper_condition::BindWrapperCondition;
use crate::flow::element::condition::chain_bind_wrapper_condition::ChainBindWrapperCondition;
use crate::flow::element::condition::{
    Condition, and_or_condition::AndOrCondition, catch_condition::CatchCondition,
    finally_condition::FinallyCondition, for_condition::ForCondition, if_condition::IfCondition,
    iterator_condition::IteratorCondition, not_condition::NotCondition,
    pre_condition::PreCondition, retry_condition::RetryCondition,
    switch_condition::SwitchCondition, then_condition::ThenCondition,
    timeout_condition::TimeoutCondition, when_condition::WhenCondition,
    while_condition::WhileCondition,
};
use crate::flow::element::fallback_node::FallbackNode;
use crate::flow::element::node::Node;
use crate::flow::entity::InstanceInfoDto;
use crate::flow::flow_bus::FlowBus;
use crate::flow::instance_id::BaseNodeInstanceIdManageSpi;
use crate::property::LiteflowConfigGetter;
use crate::util::el_regex_util::ElRegexUtil;
use md5::{Digest, Md5};
use std::cell::{Cell, Ref, RefCell};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// LiteFlow 链路 EL 构建器。
///
/// 保存当前链路标识和节点出现次数，将解析后的 EL 递归转换为 Chain、
/// Condition、Node 与 FallbackNode。对应 Java:
/// `com.yomahub.liteflow.builder.el.LiteFlowChainELBuilder`。
pub struct LiteFlowChainELBuilder {
    bus: FlowBus,
    chain: RefCell<Chain>,
    chain_id: RefCell<String>,
    /// 节点出现次数计数（NodeInstanceIdManageSpi 语义）
    occurrences: RefCell<HashMap<String, usize>>,
    /// EL 摘要一致时按 `(node_id, occurrence)` 恢复的稳定实例编号。
    restored_instance_ids: RefCell<HashMap<(String, usize), String>>,
    /// EL 变化时等待通过当前 SPI 持久化的新实例编号。
    pending_instance_infos: RefCell<Vec<InstanceInfoDto>>,
    /// 当前主体 Condition 是否启用节点实例编号。
    instance_ids_enabled: Cell<bool>,
    /// `true` 表示当前 EL 没有可复用快照，需要写入新编号。
    instance_ids_need_write: Cell<bool>,
}

impl LiteFlowChainELBuilder {
    /// 创建绑定指定 FlowBus 的构建器。
    ///
    /// Java 构造器接收 Chain；Rust 构建器接收组件和链路注册中心，以便解析节点
    /// 实例。对应 Java: `LiteFlowChainELBuilder#LiteFlowChainELBuilder`。
    pub fn new(bus: FlowBus) -> Self {
        Self {
            bus,
            chain: RefCell::new(Chain::new("", Vec::new())),
            chain_id: RefCell::new(String::new()),
            occurrences: RefCell::new(HashMap::new()),
            restored_instance_ids: RefCell::new(HashMap::new()),
            pending_instance_infos: RefCell::new(Vec::new()),
            instance_ids_enabled: Cell::new(false),
            instance_ids_need_write: Cell::new(false),
        }
    }

    /// 创建绑定指定 FlowBus 的空 Chain 构建器。
    ///
    /// Java 依赖静态 FlowBus；Rust 显式传入 `bus`，避免多个运行时相互污染。
    ///
    /// # 参数
    /// - `bus`: 接收构建结果并提供 Node/Chain 注册表的流程总线。
    ///
    /// # 返回
    /// 尚未设置 Chain ID 和 EL 的构建器。
    /// 对应 Java: `LiteFlowChainELBuilder#createChain`。
    #[must_use]
    pub fn create_chain(bus: FlowBus) -> Self {
        Self::new(bus)
    }

    /// 从已有 Chain 创建构建器。
    ///
    /// # 参数
    /// - `bus`: 接收重新编译结果的流程总线。
    /// - `chain`: 待继续设置或重新编译的 Chain。
    ///
    /// # 返回
    /// 保留原 Chain 全部元数据的构建器。
    /// 对应 Java: `LiteFlowChainELBuilder#fromChain`。
    #[must_use]
    pub fn from_chain(bus: FlowBus, chain: Chain) -> Self {
        let chain_id = chain.get_chain_id().to_string();
        Self {
            bus,
            chain: RefCell::new(chain),
            chain_id: RefCell::new(chain_id),
            occurrences: RefCell::new(HashMap::new()),
            restored_instance_ids: RefCell::new(HashMap::new()),
            pending_instance_infos: RefCell::new(Vec::new()),
            instance_ids_enabled: Cell::new(false),
            instance_ids_need_write: Cell::new(false),
        }
    }

    /// 使用已废弃的 Chain Name 设置 Chain ID。
    ///
    /// 已注册同名 Chain 时沿用其完整状态，与 Java 两阶段 Parser 预装载语义一致。
    ///
    /// # 参数
    /// - `chain_name`: Chain 名称。
    ///
    /// 对应 Java: `LiteFlowChainELBuilder#setChainName`。
    #[deprecated(note = "请使用 set_chain_id")]
    pub fn set_chain_name(&self, chain_name: impl Into<String>) -> &Self {
        let chain_name = chain_name.into();
        if let Some(chain) = self.bus.get_chain(&chain_name) {
            *self.chain.borrow_mut() = (*chain).clone();
        } else {
            self.chain.borrow_mut().set_chain_id(chain_name.clone());
        }
        *self.chain_id.borrow_mut() = chain_name;
        self
    }

    /// 设置 Chain ID。
    ///
    /// 已注册同名 Chain 时复制其状态并标记为未编译，使第二阶段能够重新组装依赖。
    ///
    /// # 参数
    /// - `chain_id`: Chain 唯一标识。
    ///
    /// 对应 Java: `LiteFlowChainELBuilder#setChainId`。
    pub fn set_chain_id(&self, chain_id: impl Into<String>) -> &Self {
        let chain_id = chain_id.into();
        if let Some(chain) = self.bus.get_chain(&chain_id) {
            let mut chain = (*chain).clone();
            chain.set_compiled(false);
            *self.chain.borrow_mut() = chain;
        } else {
            self.chain.borrow_mut().set_chain_id(chain_id.clone());
        }
        *self.chain_id.borrow_mut() = chain_id;
        self
    }

    /// 设置决策路由 EL；空白内容保持现有配置不变。
    ///
    /// # 参数
    /// - `route_el`: 返回布尔结果的路由表达式。
    ///
    /// 对应 Java: `LiteFlowChainELBuilder#setRoute`。
    pub fn set_route(&self, route_el: impl Into<String>) -> &Self {
        let route_el = route_el.into();
        if !route_el.trim().is_empty() {
            self.chain.borrow_mut().set_route_el(route_el);
        }
        self
    }

    /// 设置主体 EL，并计算规范化表达式的 MD5。
    ///
    /// # 参数
    /// - `el_str`: Chain 主体 EL；空白内容返回 FlowSystemException 对应错误。
    ///
    /// # 返回
    /// 当前构建器，便于继续链式设置。
    /// 对应 Java: `LiteFlowChainELBuilder#setEL`。
    pub fn set_el(&self, el_str: impl Into<String>) -> LFResult<&Self> {
        let el_str = el_str.into();
        if el_str.trim().is_empty() {
            return Err(LiteflowError::Custom(format!(
                "no el in this chain[{}]",
                self.chain.borrow().get_chain_id()
            )));
        }
        let normalized = ElRegexUtil::normalize(&el_str);
        let el_md5 = format!("{:x}", Md5::digest(normalized.as_bytes()));
        let mut chain = self.chain.borrow_mut();
        chain.set_el(el_str);
        chain.set_el_md5(el_md5);
        Ok(self)
    }

    /// 设置命名空间；空白内容回退到默认命名空间。
    ///
    /// # 参数
    /// - `namespace`: 路由 Chain 分组命名空间。
    ///
    /// 对应 Java: `LiteFlowChainELBuilder#setNamespace`。
    pub fn set_namespace(&self, namespace: impl Into<String>) -> &Self {
        let namespace = namespace.into();
        let namespace = if namespace.trim().is_empty() {
            ChainConstant::DEFAULT_NAMESPACE.to_string()
        } else {
            namespace
        };
        self.chain.borrow_mut().set_namespace(namespace);
        self
    }

    /// 设置 Chain 层级线程池执行器类名。
    ///
    /// # 参数
    /// - `thread_pool_executor_class`: Java 执行器构建器类名或 Rust 注册键。
    ///
    /// 对应 Java: `LiteFlowChainELBuilder#setThreadPoolExecutorClass`。
    pub fn set_thread_pool_executor_class(
        &self,
        thread_pool_executor_class: impl Into<String>,
    ) -> &Self {
        self.chain
            .borrow_mut()
            .set_thread_pool_executor_class(thread_pool_executor_class);
        self
    }

    /// 根据全局解析模式构建并注册 Chain。
    ///
    /// `PARSE_ONE_ON_FIRST_EXEC` 只预装载未编译 Chain；其他模式立即编译主体和
    /// route，完成后再原子写入 FlowBus。
    ///
    /// 对应 Java: `LiteFlowChainELBuilder#build`。
    pub fn build(&self) -> LFResult<()> {
        if LiteflowConfigGetter::get().get_parse_mode() == ParseModeEnum::ParseOneOnFirstExec {
            self.chain.borrow_mut().set_compiled(false);
            self.bus.add_chain_phase1(self.chain.borrow().clone());
            return Ok(());
        }
        self.compile_chain()
    }

    /// 立即编译并注册当前 Chain，不再读取进程级解析模式。
    ///
    /// `RuleDefinitionPlan` 已经完成 Java `PARSE_ONE_ON_FIRST_EXEC` 的定义收集；
    /// 首次执行进入物化阶段后必须无条件编译。该内部入口避免同进程其他应用上下文
    /// 改写 `LiteflowConfigGetter` 后，把当前运行时的目标链再次错误登记为未编译空链。
    ///
    /// 对应 Java: `LiteFlowChainELBuilder#buildUnCompileChain` 的最终编译阶段。
    pub(crate) fn build_immediately(&self) -> LFResult<()> {
        self.compile_chain()
    }

    /// 编译并替换一个预装载的未编译 Chain。
    ///
    /// # 参数
    /// - `bus`: Chain 所属流程总线。
    /// - `chain`: 已含 Chain ID 与 EL 的预装载对象。
    ///
    /// 对应 Java: `LiteFlowChainELBuilder#buildUnCompileChain`。
    pub fn build_un_compile_chain(bus: &FlowBus, chain: &Chain) -> LFResult<()> {
        if chain.get_el().is_none_or(|el| el.trim().is_empty()) {
            return Err(LiteflowError::Custom(format!(
                "no el content in this unCompile chain[{}]",
                chain.get_chain_id()
            )));
        }
        Self::from_chain(bus.clone(), chain.clone()).compile_chain()
    }

    /// 返回当前构建中的 Chain 只读借用。
    ///
    /// # 返回
    /// 包含已设置元数据及当前编译状态的 Chain。
    /// 对应 Java: `LiteFlowChainELBuilder#getChain`。
    #[must_use]
    pub fn get_chain(&self) -> Ref<'_, Chain> {
        self.chain.borrow()
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
        // Java validateWithEx 并非只做语法和注册表检查，而是调用 compile 执行
        // 全部 EL Operator。因此 Common Node 放进 IF/AND、错误的 Switch/Loop
        // 节点类型等，都必须在校验阶段失败。Rust 构建同一可执行对象树以复用
        // OperatorHelper 的节点类型约束，但不把临时 Chain 登记到 FlowBus。
        if let Err(error) = self.build_executable(expression) {
            return ValidationResp::fail(error);
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

    /// 从 Rust 类型化 EL 语法树构建并注册 Chain。
    ///
    /// 这是 Rust 扩展入口，不对应 Java 公共方法。语法树会先按稳定的 serde
    /// 字段顺序编码，再使用带版本前缀的 MD5 作为节点实例编号快照摘要；这样相同
    /// AST 能跨 `FlowBus` 恢复编号，AST 变化则重新生成。该入口始终立即编译，
    /// 因为类型化 AST 没有可供 `PARSE_ONE_ON_FIRST_EXEC` 延迟重解析的 EL 原文。
    ///
    /// # 参数
    /// - `chain_id`: Chain 唯一标识。
    /// - `el`: 已解析的 Rust EL 语法树。
    ///
    /// # 返回
    /// 完成 Condition 构建、实例编号持久化及原子注册时返回 `Ok(())`。
    pub fn build_parsed_chain(&self, chain_id: &str, el: El) -> LFResult<()> {
        let canonical_ast = serde_json::to_vec(&el).map_err(|error| {
            LiteflowError::Rule(format!(
                "serialize parsed EL for chain[{chain_id}] failed: {error}"
            ))
        })?;
        let mut digest_input = b"liteflow-rust-el-ast-v1:".to_vec();
        digest_input.extend_from_slice(&canonical_ast);
        let el_md5 = format!("{:x}", Md5::digest(&digest_input));

        *self.chain_id.borrow_mut() = chain_id.to_string();
        self.occurrences.borrow_mut().clear();
        self.prepare_instance_ids(chain_id, &el_md5)?;
        let body = self.build_executable(el);
        if body.is_err() {
            self.instance_ids_enabled.set(false);
            self.instance_ids_need_write.set(false);
        }
        let body = body?;
        self.persist_instance_ids(chain_id, &el_md5)?;

        let mut chain = Chain::new(chain_id, vec![body]);
        chain.set_el_md5(el_md5);
        chain.set_compiled(true);
        self.bus.add_built_chain(chain);
        Ok(())
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
        let route = self.build_route_executable(route_el)?;
        let body = self.build_executable(body_el)?;
        let mut chain = Chain::new(chain_id, vec![body]).with_namespace(namespace);
        chain.set_route_item(route);
        Ok(chain)
    }

    /// 编译当前 Chain，并让同一递归栈共享依赖环检测集合。
    fn compile_chain(&self) -> LFResult<()> {
        self.compile_chain_with_ancestors(&mut HashSet::new())
    }

    /// 先编译主体 EL 引用的未编译子 Chain，再构建当前主体与 route。
    ///
    /// Java 在 QLExpress 执行前扫描外部变量并递归调用 `buildUnCompileChain`；
    /// Rust 直接遍历类型化 AST，避免字符串误判，同时保留“子 Chain 先完成”的时序。
    fn compile_chain_with_ancestors(&self, ancestors: &mut HashSet<String>) -> LFResult<()> {
        let (chain_id, el_str, el_md5, route_el) = {
            let chain = self.chain.borrow();
            let chain_id = chain.get_chain_id().to_string();
            let el_str = chain
                .get_el()
                .filter(|el| !el.trim().is_empty())
                .ok_or_else(|| LiteflowError::Custom(format!("no el in this chain[{chain_id}]")))?
                .to_string();
            let el_md5 = chain
                .get_el_md5()
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| {
                    let normalized = ElRegexUtil::normalize(&el_str);
                    format!("{:x}", Md5::digest(normalized.as_bytes()))
                });
            (
                chain_id,
                el_str,
                el_md5,
                chain.get_route_el().map(ToOwned::to_owned),
            )
        };

        if !ancestors.insert(chain_id.clone()) {
            return Err(LiteflowError::CyclicDependency(format!(
                "cyclic chain dependency detected at chain[{chain_id}]"
            )));
        }

        let result = (|| {
            // Java 的 execute2RespWithEL 会先通过 ElRegexUtil.normalize 保留一个
            // 语句终止分号，再把结果交给 setEL/build。Rust EL 解析器只处理表达式
            // 本体，因此仅在解析阶段剔除尾部分号，Chain 中仍保留 Java 的规范化文本。
            let body_el = parse_el(el_str.trim_end_matches(';'))?;
            let mut referenced_chain_ids = Vec::new();
            collect_referenced_chain_ids(&body_el, &self.bus, &chain_id, &mut referenced_chain_ids);
            for referenced_chain_id in referenced_chain_ids {
                let Some(referenced_chain) = self.bus.get_chain(&referenced_chain_id) else {
                    continue;
                };
                if !referenced_chain.is_compiled() {
                    Self::from_chain(self.bus.clone(), (*referenced_chain).clone())
                        .compile_chain_with_ancestors(ancestors)?;
                }
            }

            // 子 Chain 已经替换到 FlowBus 后再绑定主体，确保包装器持有编译完成的 Arc。
            *self.chain_id.borrow_mut() = chain_id.clone();
            self.occurrences.borrow_mut().clear();
            self.prepare_instance_ids(&chain_id, &el_md5)?;
            let body = match self.build_executable(body_el) {
                Ok(body) => body,
                Err(error) => {
                    // Java 仅在主体 Condition 完整生成后才进入实例编号 SPI。
                    // Rust 的编号在 Node 构建过程中分配，因此失败时必须丢弃本轮
                    // 恢复表和待写 DTO，避免同一 Builder 后续构建 route 时串入。
                    self.reset_instance_id_state();
                    return Err(error);
                }
            };
            self.persist_instance_ids(&chain_id, &el_md5)?;
            let route = route_el
                .as_deref()
                .filter(|route| !route.trim().is_empty())
                .map(parse_el)
                .transpose()?
                .map(|route| self.build_route_executable(route))
                .transpose()?;

            {
                let mut chain = self.chain.borrow_mut();
                chain.set_condition_list(vec![body]);
                if let Some(route) = route {
                    chain.set_route_item(route);
                } else {
                    // Java compileChain 无论 route 是否存在都会调用
                    // setRouteItem(this.route)，空 route 必须覆盖已有 routeItem。
                    chain.clear_route_item();
                }
                chain.set_compiled(true);
            }
            self.bus.add_built_chain(self.chain.borrow().clone());
            Ok(())
        })();
        ancestors.remove(&chain_id);
        result
    }

    /// 根据配置和持久化快照准备主体 Condition 的节点实例编号。
    ///
    /// Java 只在 `enableNodeInstanceId=true` 时调用
    /// `NodeInstanceIdManageSpi#setNodesInstanceId`。Rust 在节点被放入
    /// `Arc<dyn Executable>` 前完成同一件事：摘要一致时准备恢复表，摘要不一致时
    /// 标记为重新生成。路由表达式不进入该状态，保持 Java 仅处理主体 Condition
    /// 的边界。
    ///
    /// 对应 Java: `LiteFlowChainELBuilder#setNodesInstanceId` 与
    /// `BaseNodeInstanceIdManageSpi#setNodesInstanceId`。
    fn prepare_instance_ids(&self, chain_id: &str, el_md5: &str) -> LFResult<()> {
        self.restored_instance_ids.borrow_mut().clear();
        self.pending_instance_infos.borrow_mut().clear();
        self.instance_ids_need_write.set(false);

        let enabled = LiteflowConfigGetter::get().get_enable_node_instance_id();
        self.instance_ids_enabled.set(enabled);
        if !enabled {
            return Ok(());
        }

        let spi = self
            .bus
            .instance_id_spi_holder
            .get_node_instance_id_manage_spi();
        let lines = spi.read_instance_id_file(chain_id)?;
        if !lines.first().is_some_and(|saved_md5| saved_md5 == el_md5) {
            self.instance_ids_need_write.set(true);
            return Ok(());
        }

        // Java 以 chainId、nodeId 和同名节点出现下标恢复编号；缺失项保持 None，
        // 不在摘要一致时偷偷生成新编号。
        let infos = BaseNodeInstanceIdManageSpi::parse_instance_infos(&lines)?;
        let mut restored = self.restored_instance_ids.borrow_mut();
        for info in infos {
            let (Some(saved_chain_id), Some(node_id), Some(instance_id), Some(index)) = (
                info.chain_id(),
                info.node_id(),
                info.instance_id(),
                info.index(),
            ) else {
                continue;
            };
            if saved_chain_id == chain_id {
                restored.insert((node_id.to_string(), index), instance_id.to_string());
            }
        }
        Ok(())
    }

    /// 在主体 Condition 完成构建后写入新实例编号，并关闭编号状态。
    ///
    /// 摘要一致时不会重复写文件；摘要变化时一次性写入本轮按遍历顺序生成的 DTO。
    /// 无论成功与否，后续 route 构建都不会被误计入主体实例编号。
    ///
    /// 对应 Java: `NodeInstanceIdManageSpi#writeInstanceIdFile`。
    fn persist_instance_ids(&self, chain_id: &str, el_md5: &str) -> LFResult<()> {
        let result = if self.instance_ids_enabled.get() && self.instance_ids_need_write.get() {
            let spi = self
                .bus
                .instance_id_spi_holder
                .get_node_instance_id_manage_spi();
            spi.write_instance_id_file(
                self.pending_instance_infos.borrow().as_slice(),
                el_md5,
                chain_id,
            )
        } else {
            Ok(())
        };
        self.instance_ids_enabled.set(false);
        self.instance_ids_need_write.set(false);
        result
    }

    /// 丢弃一次未完成主体编译留下的实例编号临时状态。
    fn reset_instance_id_state(&self) {
        self.occurrences.borrow_mut().clear();
        self.restored_instance_ids.borrow_mut().clear();
        self.pending_instance_infos.borrow_mut().clear();
        self.instance_ids_enabled.set(false);
        self.instance_ids_need_write.set(false);
    }

    /// 构建 Java 允许的路由类型：布尔 Node、AND/OR 或 NOT。
    fn build_route_executable(&self, route_el: El) -> LFResult<Arc<dyn Executable>> {
        if !is_route_expression(&route_el) {
            return Err(LiteflowError::RouteELInvalid(
                "the route EL can only be a boolean node, or an AND or OR expression.".to_string(),
            ));
        }
        match route_el {
            El::Node(_) | El::Mods(_, _) => {
                self.build_executable_as(route_el, NodeTypeEnum::Boolean)
            }
            other => self.build_executable(other),
        }
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
                for key in &mods.bind_override_keys {
                    clear_node_bind(&mut inner_el, key);
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
        // Java 声明式组件最终以 nodeId 作为普通 NodeComponent 进入 FlowBus；
        // Rust 在构建期把同组方法合成为完整生命周期代理。
        if let Some(decl) = self.bus.get_decl(&node_ref.id) {
            let node_id = node_ref.id.clone();
            let instance: Arc<dyn crate::core::node_component::NodeComponent> =
                Arc::new(DeclMethodComponent::for_node(decl).ok_or_else(|| {
                    LiteflowError::NodeBuild(format!(
                        "decl component[{node_id}] does not define the process method"
                    ))
                })?);
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
        let (aspects, monitor) = self.bus.hooks_snapshot();
        let hooks = NodeHooks {
            aspects,
            monitor: Some(monitor),
        };
        let node_build_life_cycles = self.bus.lifecycle.read().unwrap().node_build.clone();
        let mut node = Node::new(node_ref, instance).with_hooks(hooks);

        // Rust 的组件注册表保存可复用组件，EL Builder 为每个出现位置创建真实
        // Node。生命周期在该构建边界接收同一个可变 Node，before 的元数据修改
        // 会继续进入实例编号分配和最终 Condition。
        for life_cycle in &node_build_life_cycles {
            life_cycle.post_process_before_node_build(&mut node);
        }

        if self.instance_ids_enabled.get() {
            let node_id = node.get_id().to_string();
            let instance_id = if self.instance_ids_need_write.get() {
                let info = self
                    .bus
                    .instance_id_spi_holder
                    .get_node_instance_id_manage_spi()
                    .build_instance_info(&chain_id, &node_id, occurrence);
                let instance_id = info.instance_id().map(ToOwned::to_owned);
                self.pending_instance_infos.borrow_mut().push(info);
                instance_id
            } else {
                self.restored_instance_ids
                    .borrow()
                    .get(&(node_id, occurrence))
                    .cloned()
            };
            if let Some(instance_id) = instance_id {
                node.set_node_instance_id(instance_id);
            }
        }

        for life_cycle in &node_build_life_cycles {
            life_cycle.post_process_after_node_build(&node);
        }
        node
    }

    /// 子链包装（对应 ChainBindWrapperCondition 的构建时机）
    fn build_node_or_chain(
        &self,
        node_ref: NodeRef,
        expected_node_type: NodeTypeEnum,
    ) -> LFResult<Arc<dyn Executable>> {
        let id_no_method = node_ref.id.split('.').next().unwrap_or("");
        if self.bus.contains_node(&node_ref.id) || self.bus.get_decl(id_no_method).is_some() {
            if node_ref.condition_id.is_some() {
                return Err(LiteflowError::Parse(
                    "The caller must be Condition item".to_string(),
                ));
            }
            let node = self.build_node(node_ref)?;
            OperatorHelper::check_resolved_node(&node, expected_node_type)?;
            return Ok(Arc::new(node));
        }
        if let Some(chain) = self.bus.get_chain(&node_ref.id) {
            if expected_node_type != NodeTypeEnum::Common {
                return Err(LiteflowError::Parse("The parameter error.".to_string()));
            }
            if let Some(data) = node_ref.data.as_deref() {
                // Java DataOperator 会通过 LiteflowMetaOperator#getNodes 递归修改
                // 子链内真实共享 Node；因此该修改也必须影响子链后续独立执行。
                chain.apply_chain_cmp_data(data);
            }
            if node_ref.bind.is_empty() && node_ref.tag.is_none() {
                return Ok(chain);
            }
            if node_ref.chain_tag_wrapper {
                // Java TagOperator 对全局唯一 Chain 创建 ThenCondition，后续
                // bind/id/tag 都写在这个包装 Condition 上，不修改原 Chain。
                let mut wrapper = ThenCondition::new();
                wrapper.add_executable(chain);
                for (key, value) in node_ref.bind {
                    wrapper.put_bind_data(key, value);
                }
                if let Some(tag) = node_ref.tag {
                    wrapper.set_tag(tag);
                }
                if let Some(id) = node_ref.condition_id {
                    wrapper.set_id(id);
                }
                return Ok(Arc::new(wrapper));
            }
            // 场景3：首个属性为 bind 时，包装成 ChainBindWrapperCondition。
            let mut wrapper = ChainBindWrapperCondition::new(chain);
            for (k, v) in node_ref.bind {
                wrapper.put_bind_data(k, v);
            }
            if let Some(tag) = node_ref.tag {
                wrapper.set_tag(tag);
            }
            if let Some(id) = node_ref.condition_id {
                wrapper.set_id(id);
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

/// 判断路由 AST 是否属于 Java 允许的布尔 Node、AND/OR 或 NOT。
fn is_route_expression(el: &El) -> bool {
    match el {
        El::Node(_) | El::And(_) | El::Or(_) | El::Not(_) => true,
        // Java 在所有操作符执行完成后检查最终对象类型。retry、maxWait、
        // ignoreError 等修饰会把 Node/Condition 包装成其他 Condition，即使内部
        // 是布尔节点也不能作为 route。Node 的 tag/data/bind 已直接保存在
        // NodeRef 中，不会进入 Mods。
        El::Mods(_, _) => false,
        _ => false,
    }
}

/// 按 AST 顺序收集主体 EL 直接或嵌套引用的已注册子 Chain。
fn collect_referenced_chain_ids(
    el: &El,
    bus: &FlowBus,
    current_chain_id: &str,
    chain_ids: &mut Vec<String>,
) {
    match el {
        El::Node(node) => {
            if node.id != current_chain_id
                && bus.contains_chain(&node.id)
                && !chain_ids.contains(&node.id)
            {
                chain_ids.push(node.id.clone());
            }
        }
        El::Boolean(_) => {}
        El::Then(items) | El::And(items) | El::Or(items) | El::When { items, .. } => {
            for item in items {
                collect_referenced_chain_ids(item, bus, current_chain_id, chain_ids);
            }
        }
        El::If {
            cond,
            then,
            elifs,
            els,
        } => {
            collect_referenced_chain_ids(cond, bus, current_chain_id, chain_ids);
            collect_referenced_chain_ids(then, bus, current_chain_id, chain_ids);
            for (condition, body) in elifs {
                collect_referenced_chain_ids(condition, bus, current_chain_id, chain_ids);
                collect_referenced_chain_ids(body, bus, current_chain_id, chain_ids);
            }
            if let Some(els) = els {
                collect_referenced_chain_ids(els, bus, current_chain_id, chain_ids);
            }
        }
        El::Switch {
            node,
            targets,
            default,
        } => {
            collect_referenced_chain_ids(node, bus, current_chain_id, chain_ids);
            for target in targets {
                collect_referenced_chain_ids(target, bus, current_chain_id, chain_ids);
            }
            if let Some(default) = default {
                collect_referenced_chain_ids(default, bus, current_chain_id, chain_ids);
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
            collect_referenced_chain_ids(node, bus, current_chain_id, chain_ids);
            collect_referenced_chain_ids(body, bus, current_chain_id, chain_ids);
            if let Some(brk) = brk {
                collect_referenced_chain_ids(brk, bus, current_chain_id, chain_ids);
            }
        }
        El::ForCount { body, brk, .. } => {
            collect_referenced_chain_ids(body, bus, current_chain_id, chain_ids);
            if let Some(brk) = brk {
                collect_referenced_chain_ids(brk, bus, current_chain_id, chain_ids);
            }
        }
        El::Catch { body, do_ } => {
            collect_referenced_chain_ids(body, bus, current_chain_id, chain_ids);
            if let Some(do_) = do_ {
                collect_referenced_chain_ids(do_, bus, current_chain_id, chain_ids);
            }
        }
        El::Not(item) | El::Pre(item) | El::Fin(item) | El::Mods(item, _) => {
            collect_referenced_chain_ids(item, bus, current_chain_id, chain_ids);
        }
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
