//! 对应 FlowBus（chainMap / nodeMap 的 CopyOnWrite 语义 → DashMap）
//! 与 LiteflowMetaOperator 的链路管理。

use crate::aop::ICmpAroundAspect;
use crate::builder::el::lite_flow_chain_el_builder::LiteFlowChainELBuilder;
use crate::core::ComponentInitializer;
use crate::core::decl_component::DeclComponent;
use crate::core::node_component::NodeComponent;
use crate::core::proxy::{DeclWarpBean, LiteFlowProxyUtil};
use crate::el::{El, parse_el};
use crate::enums::{FlowParserTypeEnum, NodeTypeEnum};
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::chain::Chain;
use crate::flow::element::fallback_node::normalize_fallback_type;
use crate::flow::instance_id::{NodeInstanceIdManageSpi, NodeInstanceIdManageSpiHolder};
use crate::lifecycle::{
    LifeCycle, LifeCycleHolder, PostProcessChainBuildLifeCycle, PostProcessChainExecuteLifeCycle,
    PostProcessFlowExecuteLifeCycle, PostProcessNodeBuildLifeCycle,
    PostProcessScriptEngineInitLifeCycle,
};
use crate::monitor::{MonitorBus, MonitorFile};
use crate::parser::el::{JsonFlowElParser, XmlFlowElParser, YmlFlowElParser};
use crate::script::{ScriptKind, build_rhai_component};
use crate::spi::DeclComponentParserHolder;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};

type MonitorFileCleaner = Arc<dyn Fn() -> LFResult<()> + Send + Sync>;

/// 保存并管理 Chain、Node、脚本、生命周期和运行期扩展的流程元数据总线。
///
/// Java 使用进程级静态注册表；Rust 将同一组状态绑定到可克隆的 `FlowBus`，
/// 使多个运行时相互隔离。对应 Java:
/// `com.yomahub.liteflow.flow.FlowBus`。
#[derive(Clone)]
pub struct FlowBus {
    pub(crate) nodes: Arc<DashMap<String, Arc<dyn NodeComponent>>>,
    pub(crate) chains: Arc<DashMap<String, Arc<Chain>>>,
    /// elMd5 → chainId（2.16：execute2RespWithEL 的匿名链缓存索引）
    pub(crate) el_md5_map: Arc<DashMap<String, String>>,
    /// 声明式组件（@LiteflowCmpDefine 语义）
    pub(crate) decls: Arc<DashMap<String, Arc<dyn DeclComponent>>>,
    /// 节点类型 code → 降级组件（对应 Java FlowBus.fallbackNodeMap）。
    pub(crate) fallback_nodes: Arc<DashMap<String, Arc<dyn NodeComponent>>>,
    /// 全局切面（CmpAroundAspectHolder）
    pub(crate) aspects: Arc<std::sync::RwLock<Vec<Arc<dyn ICmpAroundAspect>>>>,
    /// 监控总线（MonitorBus）
    pub(crate) monitor: Arc<MonitorBus>,
    /// 生命周期钩子（LifeCycleHolder）
    pub(crate) lifecycle: Arc<std::sync::RwLock<LifeCycleHolder>>,
    /// 脚本节点 id → (language, kind)，供脚本热刷新保持原执行器类型。
    pub(crate) script_nodes: Arc<DashMap<String, (String, ScriptKind)>>,
    /// 实例编号 SPI
    pub(crate) instance_id_spi_holder: NodeInstanceIdManageSpiHolder,
    /// 当前总线是否已经领取过首次初始化资格。
    ///
    /// 对应 Java `FlowBus.initStat`；放入 `Arc` 后所有 `FlowBus::clone` 共享状态。
    init_stat: Arc<AtomicBool>,
    /// 当前总线创建的文件监听器弱清理动作。
    monitor_file_cleaners: Arc<std::sync::RwLock<Vec<MonitorFileCleaner>>>,
    /// 当前运行时的 MonitorFile 弱单例。
    ///
    /// Java 使用进程级 Hutool Singleton；Rust 按 FlowBus 隔离运行时，并用弱引用
    /// 避免 `FlowBus -> MonitorFile -> FlowBus` 强引用环。
    pub(crate) monitor_file_instance: Arc<Mutex<Weak<MonitorFile>>>,
}

impl Default for FlowBus {
    fn default() -> Self {
        Self::new()
    }
}

impl FlowBus {
    /// 创建一个相互隔离的空流程总线。
    ///
    /// 返回值包含空的 Chain/Node 注册表、默认节点实例 ID SPI 和生命周期容器。
    /// 对应 Java: `FlowBus` 静态初始化块。
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(DashMap::new()),
            chains: Arc::new(DashMap::new()),
            el_md5_map: Arc::new(DashMap::new()),
            decls: Arc::new(DashMap::new()),
            fallback_nodes: Arc::new(DashMap::new()),
            aspects: Arc::new(std::sync::RwLock::new(Vec::new())),
            monitor: Arc::new(MonitorBus::new()),
            lifecycle: Arc::new(std::sync::RwLock::new(LifeCycleHolder::default())),
            script_nodes: Arc::new(DashMap::new()),
            instance_id_spi_holder: NodeInstanceIdManageSpiHolder::default(),
            init_stat: Arc::new(AtomicBool::new(false)),
            monitor_file_cleaners: Arc::new(std::sync::RwLock::new(Vec::new())),
            monitor_file_instance: Arc::new(Mutex::new(Weak::new())),
        }
    }

    // ---------- 横切能力注册 ----------

    /// 注册全局切面（对应 CmpAroundAspectHolder.registerAspect）
    pub fn register_aspect(&self, aspect: Arc<dyn ICmpAroundAspect>) {
        self.aspects.write().unwrap().push(aspect);
    }
    /// 监控总线（对应 MonitorBus）
    pub fn monitor(&self) -> &Arc<MonitorBus> {
        &self.monitor
    }

    /// 登记绑定到当前总线的文件监听清理动作。
    pub(crate) fn register_monitor_file_cleaner(&self, cleaner: MonitorFileCleaner) {
        self.monitor_file_cleaners
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(cleaner);
    }
    /// 注册节点构建生命周期实现。
    ///
    /// 参数 `hook` 在节点完成构建后被调用。对应 Java:
    /// `LifeCycleHolder#addLifeCycle` 的 `PostProcessNodeBuildLifeCycle` 分支。
    pub fn register_node_build_hook(&self, hook: Arc<dyn PostProcessNodeBuildLifeCycle>) {
        self.lifecycle.write().unwrap().node_build.push(hook);
    }

    /// 注册 Chain 构建前后生命周期实现。
    ///
    /// 参数 `hook` 会在新 Chain 写入注册表前后各接收一次同一个 Chain。
    /// 对应 Java: `LifeCycleHolder#addLifeCycle` 的
    /// `PostProcessChainBuildLifeCycle` 分支。
    pub fn register_chain_build_hook(&self, hook: Arc<dyn PostProcessChainBuildLifeCycle>) {
        self.lifecycle.write().unwrap().chain_build.push(hook);
    }

    /// 注册流程执行前后生命周期实现。
    ///
    /// 参数 `hook` 作用于一次完整 Flow 执行。对应 Java:
    /// `LifeCycleHolder#addLifeCycle` 的 `PostProcessFlowExecuteLifeCycle` 分支。
    pub fn register_flow_execute_hook(&self, hook: Arc<dyn PostProcessFlowExecuteLifeCycle>) {
        self.lifecycle.write().unwrap().flow_execute.push(hook);
    }

    /// 注册 Chain 执行前后生命周期实现。
    ///
    /// 参数 `hook` 作用于主链和嵌套子链执行。对应 Java:
    /// `LifeCycleHolder#addLifeCycle` 的 `PostProcessChainExecuteLifeCycle` 分支。
    pub fn register_chain_execute_hook(&self, hook: Arc<dyn PostProcessChainExecuteLifeCycle>) {
        self.lifecycle.write().unwrap().chain_execute.push(hook);
    }
    /// 注册脚本执行器初始化完成钩子。
    ///
    /// 对应 Java `LifeCycleHolder` 中的
    /// `PostProcessScriptEngineInitLifeCycle` 容器。
    pub fn register_script_engine_init_hook(
        &self,
        hook: Arc<dyn PostProcessScriptEngineInitLifeCycle>,
    ) {
        self.lifecycle
            .write()
            .unwrap()
            .script_engine_init
            .push(hook);
    }

    /// 按真实生命周期子接口注册容器发现的生命周期对象。
    ///
    /// # 参数
    /// - `life_cycle`：实现一个或多个 LiteFlow 生命周期阶段的共享对象。
    ///
    /// Java 通过 `LifeCycleHolder#addLifeCycle` 的运行期类型判断把对象放入对应
    /// 列表；Rust 由对象安全的 `LifeCycle::register_life_cycle` 完成同等强类型
    /// 分派，并保持当前 `FlowBus` 的应用上下文隔离。
    pub fn register_life_cycle(&self, life_cycle: Arc<dyn LifeCycle>) {
        self.lifecycle
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .add_life_cycle(life_cycle);
    }

    /// 清空当前流程总线的全部生命周期钩子。
    ///
    /// 对应 Java `LifeCycleHolder#clean`。
    pub fn clean_lifecycle_hooks(&self) {
        self.lifecycle.write().unwrap().clean();
    }
    /// 替换当前总线使用的节点实例 ID 管理 SPI。
    ///
    /// 参数 `spi` 负责编号生成、快照读取和写入；后续 Builder 构建立即使用新实现。
    /// 对应 Java: `NodeInstanceIdManageSpiHolder#setSpi`。
    pub fn set_instance_id_spi(&self, spi: Arc<dyn NodeInstanceIdManageSpi>) {
        self.instance_id_spi_holder
            .set_node_instance_id_manage_spi(spi);
    }

    /// 注册声明式组件（对应 @LiteflowCmpDefine；EL 以 cmpId.method 引用方法）
    pub fn register_decl(&self, node_id: impl Into<String>, decl: Arc<dyn DeclComponent>) {
        self.decls.insert(node_id.into(), decl);
    }

    /// 校验、代理并注册声明式组件包装对象。
    ///
    /// 对应 Java `FlowBus#getNodeComponentList` 的声明式组件分支。
    pub fn try_register_decl_warp(&self, decl_warp_bean: DeclWarpBean) -> LFResult<()> {
        let parser = DeclComponentParserHolder::load_decl_component_parser();
        for parsed in parser.parse_decl_bean(decl_warp_bean)? {
            LiteFlowProxyUtil::register_decl_warp(self, parsed)?;
        }
        Ok(())
    }

    /// 注册声明式组件包装对象；失败时与普通 `register` 一样立即终止装配。
    pub fn register_decl_warp(&self, decl_warp_bean: DeclWarpBean) {
        self.try_register_decl_warp(decl_warp_bean)
            .expect("register declarative component failed");
    }

    /// 返回指定声明式组件。
    ///
    /// 参数 `node_id` 是 `cmpId.methodName` 中的组件 ID；未注册时返回 `None`。
    /// 该内部查询承接 Java `FlowBus#getNode` 的声明式组件代理解析分支。
    pub(crate) fn get_decl(&self, node_id: &str) -> Option<Arc<dyn DeclComponent>> {
        self.decls.get(node_id).map(|r| r.clone())
    }

    /// 注册降级组件。
    ///
    /// 对应 Java `FlowBus#addFallbackNode`：组件既保留自己的普通节点 id，
    /// 也按 `NodeTypeEnum` 写入唯一的降级槽位；同类型后注册者覆盖前注册者。
    pub fn register_fallback<C: NodeComponent>(
        &self,
        node_id: impl Into<String>,
        node_type: crate::enums::NodeTypeEnum,
        component: C,
    ) -> LFResult<()> {
        self.register_fallback_arc(node_id, node_type, Arc::new(component))
    }

    /// trait object 形式的降级组件注册入口。
    pub fn register_fallback_arc(
        &self,
        node_id: impl Into<String>,
        node_type: crate::enums::NodeTypeEnum,
        component: Arc<dyn NodeComponent>,
    ) -> LFResult<()> {
        let node_id = node_id.into();
        check_node_id(&node_id)?;
        let node_type = normalize_fallback_type(node_type);
        let component = ComponentInitializer::load_instance()
            .init_component(component, node_type, None, &node_id)?;
        self.nodes.insert(node_id, component.clone());
        self.fallback_nodes
            .insert(node_type.get_code().to_string(), component);
        Ok(())
    }

    /// 是否已经注册指定类型的降级组件。
    pub fn contains_fallback(&self, node_type: crate::enums::NodeTypeEnum) -> bool {
        self.fallback_nodes
            .contains_key(normalize_fallback_type(node_type).get_code())
    }

    /// 返回构建 Node 时使用的全局切面与监控器快照。
    ///
    /// 返回的拥有型切面列表和共享 MonitorBus 可安全进入异步执行对象；该内部
    /// 入口承接 Java CmpAroundAspectHolder 与 MonitorBus 的构建期读取。
    pub(crate) fn hooks_snapshot(&self) -> (Vec<Arc<dyn ICmpAroundAspect>>, Arc<MonitorBus>) {
        (self.aspects.read().unwrap().clone(), self.monitor.clone())
    }

    // ---------- 组件管理 ----------

    /// addComponent(nodeId, cmpInstance)（2.16：nodeId 必须符合变量命名规则）
    pub fn register<C: NodeComponent>(&self, node_id: impl Into<String>, cmp: C) {
        self.try_register(node_id, cmp)
            .expect("register component failed")
    }
    /// 可返回错误的注册（对应 addNode 抛 NodeIdUnIllegalException 语义）
    pub fn try_register<C: NodeComponent>(
        &self,
        node_id: impl Into<String>,
        cmp: C,
    ) -> LFResult<()> {
        self.try_register_arc(node_id, Arc::new(cmp))
    }
    /// 注册已经类型擦除的线程安全节点组件。
    ///
    /// 参数 `node_id` 是 EL 中使用的节点 ID，`cmp` 是待初始化的组件实例；校验
    /// 失败时保持 Java 注册入口的立即失败语义。对应 Java: `FlowBus#addNode`。
    pub fn register_arc(&self, node_id: impl Into<String>, cmp: Arc<dyn NodeComponent>) {
        self.try_register_arc(node_id, cmp)
            .expect("register component failed");
    }
    /// 可返回错误的 Arc 组件注册，供 Rust 原生动态 builder 使用。
    pub fn try_register_arc(
        &self,
        node_id: impl Into<String>,
        cmp: Arc<dyn NodeComponent>,
    ) -> LFResult<()> {
        let id = node_id.into();
        check_node_id(&id)?;
        let cmp = ComponentInitializer::load_instance().init_inferred_component(
            cmp,
            crate::enums::NodeTypeEnum::Common,
            None,
            &id,
        )?;
        self.nodes.insert(id, cmp);
        Ok(())
    }

    /// 添加由宿主管理的节点组件。
    ///
    /// Rust 没有 Java 运行期类反射，因此组件必须通过 `node_type()` 暴露其类型；
    /// `liteflow-derive` 和已初始化组件都会提供该元数据。参数 `node_id` 与
    /// `node_component` 对应 Java 同名参数。对应 Java: `FlowBus#addManagedNode`。
    pub fn add_managed_node(
        &self,
        node_id: impl Into<String>,
        node_component: Arc<dyn NodeComponent>,
    ) -> LFResult<()> {
        let node_id = node_id.into();
        let node_type = node_component.node_type().ok_or_else(|| {
            LiteflowError::NodeBuild(format!("node type is null for node[{node_id}]"))
        })?;
        self.add_node(node_id, None, node_type, node_component)
    }

    /// 按显式节点类型初始化并注册组件。
    ///
    /// Rust 用组件实例替代 Java 的 `Class<?> cmpClazz` 反射构造。参数
    /// `node_id`、`name`、`node_type` 与 Java 语义一一对应。
    /// 对应 Java: `FlowBus#addNode`。
    pub fn add_node(
        &self,
        node_id: impl Into<String>,
        name: Option<&str>,
        node_type: NodeTypeEnum,
        node_component: Arc<dyn NodeComponent>,
    ) -> LFResult<()> {
        let node_id = node_id.into();
        check_node_id(&node_id)?;
        let component = ComponentInitializer::load_instance().init_component(
            node_component,
            node_type,
            name,
            &node_id,
        )?;
        self.nodes.insert(node_id, component);
        Ok(())
    }

    /// 插入已经由 `ComponentInitializer` 注入元数据的组件。
    pub(crate) fn insert_initialized_arc(
        &self,
        node_id: impl Into<String>,
        component: Arc<dyn NodeComponent>,
    ) -> LFResult<()> {
        let node_id = node_id.into();
        check_node_id(&node_id)?;
        self.nodes.insert(node_id, component);
        Ok(())
    }
    /// removeComponent
    pub fn unregister(&self, node_id: &str) {
        self.nodes.remove(node_id);
        self.script_nodes.remove(node_id);
    }
    /// 返回指定节点是否已经注册。
    ///
    /// 参数 `node_id` 是待查询节点 ID。对应 Java: `FlowBus#containNode`。
    #[must_use]
    pub fn contains_node(&self, node_id: &str) -> bool {
        self.nodes.contains_key(node_id)
    }

    /// 判断节点是否存在。对应 Java: `FlowBus#containNode`。
    #[must_use]
    pub fn contain_node(&self, node_id: &str) -> bool {
        self.contains_node(node_id)
    }

    /// 返回节点注册表的线程安全快照。
    ///
    /// Java 返回全局可变 Map；Rust 返回克隆后的 `Arc` 快照，避免调用方绕过
    /// 初始化和生命周期逻辑修改 `DashMap`。对应 Java: `FlowBus#getNodeMap`。
    #[must_use]
    pub fn get_node_map(&self) -> HashMap<String, Arc<dyn NodeComponent>> {
        self.nodes
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// 移除节点注册但不主动卸载脚本缓存。
    ///
    /// 返回是否确实移除了节点。对应 Java: `FlowBus#removeNode`。
    pub fn remove_node(&self, node_id: &str) -> bool {
        self.nodes.remove(node_id).is_some()
    }

    /// 返回指定节点组件的共享实例。
    ///
    /// 参数 `node_id` 是 EL 节点 ID；未注册时返回 `None`。对应 Java:
    /// `FlowBus#getNode`。
    pub(crate) fn get_node(&self, node_id: &str) -> Option<Arc<dyn NodeComponent>> {
        self.nodes.get(node_id).map(|r| r.clone())
    }

    // ---------- 链路管理（LiteflowMetaOperator） ----------

    /// LiteFlowChainELBuilder.createChain().setChainId().setEL()
    pub fn add_chain(&self, chain_id: impl Into<String>, el: &str) -> LFResult<()> {
        let id = chain_id.into();
        let builder = LiteFlowChainELBuilder::create_chain(self.clone());
        builder.set_chain_id(id);
        builder.set_el(el)?;
        builder.build()
    }

    /// 以 Rust 类型化语法树构建链路。
    ///
    /// 该 Rust 扩展入口委托给统一 Builder，完整执行节点实例编号恢复/持久化、
    /// Chain 注册和构建后生命周期。类型化 AST 不含 EL 原文，因此始终立即编译。
    ///
    /// # 参数
    /// - `chain_id`: Chain 唯一标识。
    /// - `el`: 已解析的 EL 语法树。
    ///
    /// # 返回
    /// 构建和原子注册成功时返回 `Ok(())`。
    pub fn add_chain_el(&self, chain_id: impl Into<String>, el: El) -> LFResult<()> {
        let id = chain_id.into();
        LiteFlowChainELBuilder::new(self.clone()).build_parsed_chain(&id, el)
    }

    /// 第一阶段预装载已经创建的链，不触发生命周期与 EL MD5 登记。
    ///
    /// 对应 Java: `FlowBus#addChainPhase1`。
    pub fn add_chain_phase1(&self, chain: Chain) {
        self.chains
            .insert(chain.get_chain_id().to_string(), Arc::new(chain));
    }

    /// 由已构建的 Chain 完成生命周期调用并原子写入注册表。
    ///
    /// 参数 `chain` 已包含可执行 Condition 和 EL 元数据。before 回调发生在写入
    /// 前并可修改 Chain 元数据；after 回调发生在写入后，且两阶段接收同一个
    /// Chain。
    /// 对应 Java: `FlowBus#addChain(Chain)`。
    pub fn add_built_chain(&self, mut chain: Chain) {
        let hooks = self
            .lifecycle
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .chain_build
            .clone();
        for hook in &hooks {
            hook.post_process_before_chain_build(&mut chain);
        }

        let chain = Arc::new(chain);
        let id = chain.id.clone();
        if let Some(md5) = chain.get_el_md5().map(ToOwned::to_owned) {
            self.el_md5_map.insert(md5, id.clone());
        }
        self.chains.insert(id, Arc::clone(&chain));

        for hook in &hooks {
            hook.post_process_after_chain_build(&chain);
        }
    }

    /// 构建匿名链路并登记 EL MD5 索引。
    ///
    /// # 参数
    /// - `chain_id`: 首次执行动态 EL 时生成的匿名 Chain ID。
    /// - `normalized_el`: 经 `ElRegexUtil::normalize` 处理并保留尾部分号的 EL。
    /// - `el_md5`: 调用方对规范化 EL 计算的 MD5，用于校验缓存键一致性。
    ///
    /// # 返回
    /// Builder 编译、实例编号持久化和 Chain 注册成功时返回 `Ok(())`；摘要不一致
    /// 或 EL 无效时返回对应错误。
    /// 对应 Java: `FlowExecutor#execute2RespWithEL`。
    pub fn add_chain_anonymous(
        &self,
        chain_id: &str,
        normalized_el: &str,
        el_md5: String,
    ) -> LFResult<()> {
        let builder = LiteFlowChainELBuilder::create_chain(self.clone());
        builder.set_chain_id(chain_id);
        builder.set_el(normalized_el)?;

        // 调用方与 Builder 都按 Java ElRegexUtil.normalize 计算摘要；显式校验可防止
        // 匿名链缓存键与 Chain 自身摘要分裂，避免后续错误复用另一条链。
        let builder_el_md5 = builder
            .get_chain()
            .get_el_md5()
            .map(ToOwned::to_owned)
            .ok_or_else(|| {
                LiteflowError::Parse(format!(
                    "anonymous chain[{chain_id}] does not contain an EL MD5"
                ))
            })?;
        if builder_el_md5 != el_md5 {
            return Err(LiteflowError::Parse(format!(
                "anonymous chain[{chain_id}] EL MD5 mismatch"
            )));
        }

        // 对应 Java FlowExecutor#execute2RespWithEL：匿名 EL 与普通 Chain 共用
        // LiteFlowChainELBuilder，因此完整触发节点实例编号、生命周期和原子注册。
        builder.build()
    }

    /// getChainIdByElMd5（2.16）
    pub fn get_chain_id_by_el_md5(&self, el_md5: &str) -> Option<String> {
        self.el_md5_map.get(el_md5).map(|r| r.clone())
    }

    /// reloadChain：热刷新
    pub fn reload_chain(&self, chain_id: &str, el: &str) -> LFResult<()> {
        self.reload_chain_with_route(chain_id, el, None)
    }

    /// 使用可选 route 热刷新或创建 Chain。
    ///
    /// Java 的三参数重载不会要求 Chain 预先存在；`route=None` 时，已有 Chain
    /// 保留原 route，新 Chain 不设置 route。参数分别对应 Java `chainId`、
    /// `elContent` 与 `routeContent`。
    /// 对应 Java: `FlowBus#reloadChain(String,String,String)`。
    pub fn reload_chain_with_route(
        &self,
        chain_id: &str,
        el: &str,
        route: Option<&str>,
    ) -> LFResult<()> {
        let builder = LiteFlowChainELBuilder::create_chain(self.clone());
        builder.set_chain_id(chain_id);
        builder.set_el(el)?;
        if let Some(route) = route {
            builder.set_route(route);
        }
        builder.build()
    }

    /// 从元数据注册表中移除指定 Chain，并同步清除匿名 EL 摘要索引。
    ///
    /// 参数 `chain_id` 是待删除 Chain ID；确实删除时返回 `true`，不存在时返回
    /// `false`。对应 Java: `FlowBus#removeChain(String)`。
    pub fn remove_chain(&self, chain_id: &str) -> bool {
        if let Some((_, chain)) = self.chains.remove(chain_id) {
            // 2.16：移除 chain 时同步清理 elMd5 索引
            if let Some(md5) = chain.get_el_md5() {
                self.el_md5_map.remove(md5);
            }
            true
        } else {
            false
        }
    }

    /// 创建 Chain 缓存淘汰时使用的弱引用清理函数。
    ///
    /// 返回闭包只持有 `chains` 与 `el_md5_map` 的 `Weak` 引用，避免
    /// `FlowBus -> LifeCycleHolder -> ChainCacheLifeCycle -> FlowBus` 形成引用环。
    /// 清理行为对应 Java `ChainCacheLifeCycle#cleanChain`：Rust 删除已物化 Chain，
    /// `PARSE_ONE_ON_FIRST_EXEC` 的 `RuleDefinitionPlan` 会在下次执行时重新构建。
    #[must_use]
    pub fn chain_cache_cleaner(&self) -> Arc<dyn Fn(&str) + Send + Sync> {
        let chains = Arc::downgrade(&self.chains);
        let el_md5_map = Arc::downgrade(&self.el_md5_map);
        Arc::new(move |chain_id| {
            let Some(chains) = chains.upgrade() else {
                return;
            };
            if let Some((_, chain)) = chains.remove(chain_id)
                && let (Some(el_md5), Some(el_md5_map)) = (chain.get_el_md5(), el_md5_map.upgrade())
            {
                el_md5_map.remove(el_md5);
            }
        })
    }

    /// 返回指定 Chain 是否已经注册。
    ///
    /// 参数 `chain_id` 是待查询 Chain ID。对应 Java: `FlowBus#containChain`。
    #[must_use]
    pub fn contains_chain(&self, chain_id: &str) -> bool {
        self.chains.contains_key(chain_id)
    }

    /// 判断链是否存在。对应 Java: `FlowBus#containChain`。
    #[must_use]
    pub fn contain_chain(&self, chain_id: &str) -> bool {
        self.contains_chain(chain_id)
    }

    /// 原子领取首次初始化资格。
    ///
    /// 仅第一次调用返回 `true`；`clear_stat` 后可再次领取。
    /// 对应 Java: `FlowBus#needInit`。
    pub fn need_init(&self) -> bool {
        self.init_stat
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    /// 返回链注册表的线程安全快照。
    ///
    /// 对应 Java: `FlowBus#getChainMap`。
    #[must_use]
    pub fn get_chain_map(&self) -> HashMap<String, Arc<Chain>> {
        self.chains
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }
    /// 返回当前全部 Chain ID 的线程安全快照。
    ///
    /// 返回顺序不作保证；该 Rust 便利入口读取的事实来源与 Java
    /// `FlowBus#getChainMap` 相同。
    #[must_use]
    pub fn chain_ids(&self) -> Vec<String> {
        self.chains.iter().map(|r| r.key().clone()).collect()
    }

    /// 返回指定 Chain 的共享实例。
    ///
    /// 参数 `chain_id` 是 Chain 唯一 ID；未注册时返回 `None`。对应 Java:
    /// `FlowBus#getChain`。
    pub(crate) fn get_chain(&self, chain_id: &str) -> Option<Arc<Chain>> {
        self.chains.get(chain_id).map(|r| r.clone())
    }

    /// 注册脚本节点（对应规则文件 nodes 中的 script 节点；当前支持 language="rhai"）
    pub fn register_script(
        &self,
        node_id: impl Into<String>,
        language: &str,
        script: &str,
    ) -> LFResult<()> {
        self.register_script_typed(node_id, language, ScriptKind::Common, script)
    }

    /// 按类型注册脚本节点（script / boolean_script / switch_script / for_script / iterator_script）
    pub fn register_script_typed(
        &self,
        node_id: impl Into<String>,
        language: &str,
        kind: ScriptKind,
        script: &str,
    ) -> LFResult<()> {
        self.register_script_typed_named(node_id, None, language, kind, script)
    }

    fn register_script_typed_named(
        &self,
        node_id: impl Into<String>,
        name: Option<&str>,
        language: &str,
        kind: ScriptKind,
        script: &str,
    ) -> LFResult<()> {
        let id = node_id.into();
        let component = match language {
            "rhai" => {
                let component = build_rhai_component(&id, kind, script)?;
                self.run_script_engine_init_hooks(language);
                component
            }
            other => {
                let component =
                    crate::script::ScriptExecutorFactory::build(other, &id, kind, script)?;
                self.run_script_engine_init_hooks(other);
                component
            }
        };
        let node_type = node_type_for_script_kind(kind);
        let component = ComponentInitializer::load_instance()
            .init_component(component, node_type, name, &id)?;
        self.nodes.insert(id.clone(), component);
        self.script_nodes
            .insert(id.clone(), (language.to_string(), kind));
        Ok(())
    }

    /// 添加并编译脚本节点。
    ///
    /// `PARSE_ONE_ON_FIRST_EXEC` 的延迟构建由 Rust `RuleDefinitionPlan` 在更外层
    /// 处理；一旦调用本方法，就与 Java 非延迟分支一样完成真实编译和原子注册。
    /// 对应 Java: `FlowBus#addScriptNode`。
    pub fn add_script_node(
        &self,
        node_id: impl Into<String>,
        name: Option<&str>,
        node_type: NodeTypeEnum,
        script: &str,
        language: &str,
    ) -> LFResult<()> {
        let kind = script_kind_for_node_type(node_type)?;
        self.register_script_typed_named(node_id, name, language, kind, script)
    }

    /// 添加并立即编译脚本节点。对应 Java: `FlowBus#addScriptNodeAndCompile`。
    pub fn add_script_node_and_compile(
        &self,
        node_id: impl Into<String>,
        name: Option<&str>,
        node_type: NodeTypeEnum,
        script: &str,
        language: &str,
    ) -> LFResult<()> {
        self.add_script_node(node_id, name, node_type, script, language)
    }

    /// 编译并替换一个脚本节点。
    ///
    /// Rust 不保留 Java 可变 `Node` 半成品，故显式接收其五项元数据。
    /// 对应 Java: `FlowBus#compileScriptNode`。
    pub fn compile_script_node(
        &self,
        node_id: impl Into<String>,
        name: Option<&str>,
        node_type: NodeTypeEnum,
        script: &str,
        language: &str,
    ) -> LFResult<()> {
        self.add_script_node_and_compile(node_id, name, node_type, script, language)
    }

    /// 返回指定类型的降级组件。
    ///
    /// 对应 Java: `FlowBus#getFallBackNode`。
    #[must_use]
    pub fn get_fall_back_node(&self, node_type: NodeTypeEnum) -> Option<Arc<dyn NodeComponent>> {
        self.fallback_nodes
            .get(normalize_fallback_type(node_type).get_code())
            .map(|entry| entry.clone())
    }

    /// 卸载脚本编译产物并移除节点。
    ///
    /// 非脚本节点或不存在的节点返回 `Ok(false)`。对应 Java:
    /// `FlowBus#unloadScriptNode`。
    pub fn unload_script_node(&self, node_id: &str) -> LFResult<bool> {
        let Some(component) = self.get_node(node_id) else {
            return Ok(false);
        };
        if !self.script_nodes.contains_key(node_id) {
            return Ok(false);
        }
        // Java ScriptExecutor#unLoad 是 void；即使缓存已由 cleanScriptCache 清空，
        // unloadScriptNode 仍继续从 nodeMap 删除元数据。
        let _ = component.unload_script(node_id)?;
        self.nodes.remove(node_id);
        self.script_nodes.remove(node_id);
        Ok(true)
    }

    /// 卸载当前总线内全部脚本编译缓存。
    ///
    /// Rust 插件构建器是应用配置而非 Java ServiceLoader 实例缓存，因此保留语言
    /// 注册，只清理真实组件编译产物。对应 Java: `FlowBus#cleanScriptCache`。
    pub fn clean_script_cache(&self) -> LFResult<()> {
        let node_ids = self
            .script_nodes
            .iter()
            .map(|entry| entry.key().clone())
            .collect::<Vec<_>>();
        for node_id in node_ids {
            // Java 只清理 ScriptExecutor 的编译缓存，不从 nodeMap 删除脚本节点。
            // `cleanCache` 会在本方法返回后统一清空 Node；单独调用本方法时则保留
            // 元数据，允许 reloadScript 重新装载。
            if let Some(component) = self.get_node(&node_id) {
                let _ = component.unload_script(&node_id)?;
            }
        }
        Ok(())
    }

    /// 清空当前总线拥有的链、节点、降级组件和 EL 索引。
    ///
    /// 清理前先停止文件监听并卸载脚本编译产物。对应 Java:
    /// `FlowBus#cleanCache`。
    pub fn clean_cache(&self) -> LFResult<()> {
        self.clean_monitor_file()?;
        self.clean_script_cache()?;
        self.chains.clear();
        self.nodes.clear();
        self.fallback_nodes.clear();
        self.el_md5_map.clear();
        Ok(())
    }

    /// 停止并清空绑定到当前总线的全部文件监听器。
    ///
    /// 清理器真实终止 Tokio 任务并删除路径状态；已经析构的监听器由弱引用安全
    /// 跳过。对应 Java: `FlowBus#cleanMonitorFile`。
    pub fn clean_monitor_file(&self) -> LFResult<()> {
        let cleaners = {
            let mut guard = self
                .monitor_file_cleaners
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            std::mem::take(&mut *guard)
        };
        for cleaner in cleaners {
            cleaner()?;
        }
        Ok(())
    }

    /// 使用指定 EL 规则格式刷新当前总线元数据。
    ///
    /// 返回成功装载的链 id；Java `void` API 的副作用保持不变，同时让 Rust 调用方
    /// 可观测刷新结果。对应 Java: `FlowBus#refreshFlowMetaData`。
    pub fn refresh_flow_meta_data(
        &self,
        parser_type: FlowParserTypeEnum,
        content: &str,
    ) -> LFResult<Vec<String>> {
        let contents = [content.to_string()];
        match parser_type {
            FlowParserTypeEnum::TypeElXml => XmlFlowElParser::new(self.clone()).parse(&contents),
            FlowParserTypeEnum::TypeElJson => JsonFlowElParser::new(self.clone()).parse(&contents),
            FlowParserTypeEnum::TypeElYml => YmlFlowElParser::new(self.clone()).parse(&contents),
            _ => Ok(Vec::new()),
        }
    }

    /// 重置首次初始化状态。对应 Java: `FlowBus#clearStat`。
    pub fn clear_stat(&self) {
        self.init_stat.store(false, Ordering::Release);
    }

    /// 以原语言和节点类别热刷新脚本。
    ///
    /// 对应 Java `FlowBus#reloadScript`：先完整构建新组件，成功后再原子替换。
    pub fn reload_script(&self, node_id: &str, script: &str) -> LFResult<()> {
        let Some((language, kind)) = self.script_nodes.get(node_id).map(|entry| entry.clone())
        else {
            // Java 对不存在节点和普通节点直接返回，不抛出 NodeNotFound。
            return Ok(());
        };
        self.register_script_typed(node_id, &language, kind, script)?;

        // 已编译 Chain 中的 Node 持有旧组件 Arc；脚本组件替换后重建 Chain，
        // 对齐 Java 通过共享 ScriptExecutor 缓存让既有 Chain 立即看到新脚本。
        let chain_sources = self
            .chains
            .iter()
            .filter_map(|entry| {
                entry
                    .value()
                    .get_el()
                    .map(|el| (entry.key().clone(), el.to_string()))
            })
            .collect::<Vec<_>>();
        for (chain_id, el) in chain_sources {
            self.reload_chain(&chain_id, &el)?;
        }
        Ok(())
    }

    /// 在脚本组件真实构建完成后调用初始化生命周期。
    ///
    /// 先克隆钩子快照，避免用户回调再次注册或清理钩子时造成锁重入。
    fn run_script_engine_init_hooks(&self, language: &str) {
        let hooks = self.lifecycle.read().unwrap().script_engine_init.clone();
        for hook in hooks {
            hook.post_process_after_script_engine_init(language);
        }
    }

    /// 构建并注册包含 route 判断与主体 EL 的决策表 Chain。
    ///
    /// # 参数
    /// - `chain_id`: Chain 唯一标识。
    /// - `namespace`: 决策路由命名空间。
    /// - `route_el`: 只允许布尔节点、AND、OR 或 NOT 的路由表达式。
    /// - `body_el`: 路由命中后执行的主体表达式。
    ///
    /// # 返回
    /// 标准 Builder 完成主体实例编号、route 校验和注册时返回 `Ok(())`。
    /// 对应 Java: `LiteFlowChainELBuilder#setRoute` 与 `#setEL`。
    pub fn add_route_chain(
        &self,
        chain_id: impl Into<String>,
        namespace: &str,
        route_el: &str,
        body_el: &str,
    ) -> LFResult<()> {
        let id = chain_id.into();
        let builder = LiteFlowChainELBuilder::create_chain(self.clone());
        builder.set_chain_id(id);
        builder.set_namespace(namespace);
        builder.set_route(route_el);
        builder.set_el(body_el)?;
        builder.build()
    }

    /// EL 语法校验（validate）
    pub fn validate_el(el: &str) -> LFResult<()> {
        parse_el(el).map(|_| ())
    }
}

fn script_kind_for_node_type(node_type: NodeTypeEnum) -> LFResult<ScriptKind> {
    match node_type {
        NodeTypeEnum::Script => Ok(ScriptKind::Common),
        NodeTypeEnum::BooleanScript | NodeTypeEnum::IfScript => Ok(ScriptKind::Boolean),
        NodeTypeEnum::SwitchScript => Ok(ScriptKind::Switch),
        NodeTypeEnum::ForScript | NodeTypeEnum::WhileScript | NodeTypeEnum::BreakScript => {
            Ok(ScriptKind::For)
        }
        other => Err(LiteflowError::NodeTypeError {
            node: String::new(),
            expect: "script node type".to_string(),
            actual: other.get_code().to_string(),
        }),
    }
}

fn node_type_for_script_kind(kind: ScriptKind) -> NodeTypeEnum {
    match kind {
        ScriptKind::Common => NodeTypeEnum::Script,
        ScriptKind::Boolean => NodeTypeEnum::BooleanScript,
        ScriptKind::Switch => NodeTypeEnum::SwitchScript,
        ScriptKind::For => NodeTypeEnum::ForScript,
        // Java v2.16 没有 ITERATOR_SCRIPT 枚举；Rust 扩展仍以迭代节点行为执行。
        ScriptKind::Iterator => NodeTypeEnum::Iterator,
    }
}

/// nodeId 合法性校验（2.16：对应 QlExpressUtils.checkVariableName +
/// NodeIdUnIllegalException；不能以数字开头，只能由字母/数字/下划线/$ 组成）
fn check_node_id(node_id: &str) -> LFResult<()> {
    if crate::util::QlExpressUtils::check_variable_name(node_id) {
        Ok(())
    } else {
        Err(LiteflowError::NodeIdUnIllegal(node_id.to_string()))
    }
}

// ---------- 执行便捷入口（委托给 FlowExecutor，对应 FlowExecutorHolder 的便捷用法） ----------
use crate::core::flow_executor::FlowExecutor;
use crate::flow::liteflow_response::LiteflowResponse;
use serde::Serialize;
use serde_json::Value;
use std::any::Any as StdAny;
use std::time::Duration;

impl FlowBus {
    /// 创建绑定当前流程总线的执行器。
    ///
    /// 返回的执行器与当前 `FlowBus` 共享 Chain、Node、监控及生命周期状态。
    /// 对应 Java: `FlowExecutorHolder#loadInstance` 的便捷调用语义。
    #[must_use]
    pub fn executor(&self) -> FlowExecutor {
        FlowExecutor::new(self.clone())
    }
    /// execute2Resp(chainId)
    pub async fn execute(&self, chain_id: &str) -> LiteflowResponse {
        self.executor().execute(chain_id).await
    }
    /// execute2Resp(chainId, requestData)
    pub async fn execute_with_data(
        &self,
        chain_id: &str,
        input: impl Serialize,
    ) -> LiteflowResponse {
        self.executor().execute_with_data(chain_id, input).await
    }
    /// execute2Resp(chainId, requestData, contextBeanArray)
    pub async fn execute_with(
        &self,
        chain_id: &str,
        input: Value,
        beans: Vec<(String, Arc<dyn StdAny + Send + Sync>)>,
    ) -> LiteflowResponse {
        self.executor().execute_with(chain_id, input, beans).await
    }

    /// 异步提交链路并返回 Tokio 任务句柄。
    ///
    /// 对应 Java `FlowExecutor#execute2Future`，实际并发由主执行器控制。
    pub fn execute_future_with_option(
        &self,
        chain_id: impl Into<String>,
        input: Value,
        option: crate::core::ExecuteOption,
    ) -> LFResult<tokio::task::JoinHandle<LiteflowResponse>> {
        self.executor()
            .execute_future_with_option(chain_id, input, option)
    }
    /// executeRouteChain(namespace, param)
    pub async fn execute_route_chain(
        &self,
        namespace: Option<&str>,
        input: impl Serialize,
    ) -> LFResult<Vec<LiteflowResponse>> {
        self.executor().execute_route_chain(namespace, input).await
    }
    /// executeRouteChainWithRid(namespace, param, requestId)
    pub async fn execute_route_chain_with_rid(
        &self,
        namespace: Option<&str>,
        input: impl Serialize,
        request_id: impl Into<String>,
    ) -> LFResult<Vec<LiteflowResponse>> {
        self.executor()
            .execute_route_chain_with_rid(namespace, input, request_id)
            .await
    }

    /// execute2Resp(chainId, requestData, timeout, unit)
    pub async fn execute_timeout(
        &self,
        chain_id: &str,
        input: impl Serialize,
        timeout: Duration,
    ) -> LiteflowResponse {
        self.executor()
            .execute_timeout(chain_id, input, timeout)
            .await
    }

    /// execute2RespWithEL(elStr)（2.16：直接执行 EL 表达式）
    pub async fn execute_with_el(&self, el_str: &str) -> LiteflowResponse {
        self.executor().execute_with_el(el_str).await
    }
    /// execute2RespWithEL(elStr, param)
    pub async fn execute_with_el_data(
        &self,
        el_str: &str,
        input: impl Serialize,
    ) -> LiteflowResponse {
        self.executor().execute_with_el_data(el_str, input).await
    }
    /// execute2Resp(chainId, requestData, ExecuteOption)（2.16）
    pub async fn execute_with_option(
        &self,
        chain_id: &str,
        input: Value,
        option: crate::core::execute_option::ExecuteOption,
    ) -> LiteflowResponse {
        self.executor()
            .execute_with_option(chain_id, input, option)
            .await
    }
    /// execute2RespWithRid（2.16：组件内以同一 requestId 调子链）
    pub async fn execute_with_rid(
        &self,
        chain_id: &str,
        input: Value,
        request_id: impl Into<String>,
    ) -> LiteflowResponse {
        self.executor()
            .execute_with_rid(
                chain_id,
                input,
                request_id,
                Vec::<(String, Arc<dyn StdAny + Send + Sync>)>::new(),
            )
            .await
    }
}
