//! 对应 FlowBus（chainMap / nodeMap 的 CopyOnWrite 语义 → DashMap）
//! 与 LiteflowMetaOperator 的链路管理。

use crate::aop::CmpAroundAspect;
use crate::builder::el::builder::LiteFlowChainELBuilder;
use crate::core::decl_component::DeclComponent;
use crate::instance_id::{DefaultNodeInstanceIdManageSpi, NodeInstanceIdManageSpi};
use crate::lifecycle::{
    PostProcessChainBuildLifeCycle, PostProcessChainExecuteLifeCycle,
    PostProcessFlowExecuteLifeCycle, PostProcessNodeBuildLifeCycle,
};
use crate::monitor::MonitorBus;
use crate::script::script_component::{ScriptComponent, ScriptKind};
use crate::core::node_component::NodeComponent;
use crate::el::{parse_el, El};
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::chain::Chain;
use dashmap::DashMap;
use std::sync::Arc;

/// 流程总线
#[derive(Clone)]
pub struct FlowBus {
    pub(crate) nodes: Arc<DashMap<String, Arc<dyn NodeComponent>>>,
    pub(crate) chains: Arc<DashMap<String, Arc<Chain>>>,
    /// 声明式组件（@LiteflowCmpDefine 语义）
    pub(crate) decls: Arc<DashMap<String, Arc<dyn DeclComponent>>>,
    /// 全局切面（CmpAroundAspectHolder）
    pub(crate) aspects: Arc<std::sync::RwLock<Vec<Arc<dyn CmpAroundAspect>>>>,
    /// 监控总线（MonitorBus）
    pub(crate) monitor: Arc<MonitorBus>,
    /// 生命周期钩子（LifeCycleHolder）
    pub(crate) lifecycle: Arc<std::sync::RwLock<LifeCycleHooks>>,
    /// 实例编号 SPI
    pub(crate) instance_id_spi: Arc<std::sync::RwLock<Arc<dyn NodeInstanceIdManageSpi>>>,
}

/// 生命周期钩子集合
#[derive(Default)]
pub struct LifeCycleHooks {
    pub node_build: Vec<Arc<dyn PostProcessNodeBuildLifeCycle>>,
    pub chain_build: Vec<Arc<dyn PostProcessChainBuildLifeCycle>>,
    pub flow_execute: Vec<Arc<dyn PostProcessFlowExecuteLifeCycle>>,
    pub chain_execute: Vec<Arc<dyn PostProcessChainExecuteLifeCycle>>,
}

impl Default for FlowBus {
    fn default() -> Self {
        Self::new()
    }
}

impl FlowBus {
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(DashMap::new()),
            chains: Arc::new(DashMap::new()),
            decls: Arc::new(DashMap::new()),
            aspects: Arc::new(std::sync::RwLock::new(Vec::new())),
            monitor: Arc::new(MonitorBus::new()),
            lifecycle: Arc::new(std::sync::RwLock::new(LifeCycleHooks::default())),
            instance_id_spi: Arc::new(std::sync::RwLock::new(
                Arc::new(DefaultNodeInstanceIdManageSpi::default())
                    as Arc<dyn NodeInstanceIdManageSpi>,
            )),
        }
    }

    // ---------- 横切能力注册 ----------

    /// 注册全局切面（对应 CmpAroundAspectHolder.registerAspect）
    pub fn register_aspect(&self, aspect: Arc<dyn CmpAroundAspect>) {
        self.aspects.write().unwrap().push(aspect);
    }
    /// 监控总线（对应 MonitorBus）
    pub fn monitor(&self) -> &Arc<MonitorBus> {
        &self.monitor
    }
    pub fn register_node_build_hook(&self, h: Arc<dyn PostProcessNodeBuildLifeCycle>) {
        self.lifecycle.write().unwrap().node_build.push(h);
    }
    pub fn register_chain_build_hook(&self, h: Arc<dyn PostProcessChainBuildLifeCycle>) {
        self.lifecycle.write().unwrap().chain_build.push(h);
    }
    pub fn register_flow_execute_hook(&self, h: Arc<dyn PostProcessFlowExecuteLifeCycle>) {
        self.lifecycle.write().unwrap().flow_execute.push(h);
    }
    pub fn register_chain_execute_hook(&self, h: Arc<dyn PostProcessChainExecuteLifeCycle>) {
        self.lifecycle.write().unwrap().chain_execute.push(h);
    }
    pub fn set_instance_id_spi(&self, spi: Arc<dyn NodeInstanceIdManageSpi>) {
        *self.instance_id_spi.write().unwrap() = spi;
    }

    /// 注册声明式组件（对应 @LiteflowCmpDefine；EL 以 cmpId.method 引用方法）
    pub fn register_decl(&self, node_id: impl Into<String>, decl: Arc<dyn DeclComponent>) {
        self.decls.insert(node_id.into(), decl);
    }
    pub(crate) fn get_decl(&self, node_id: &str) -> Option<Arc<dyn DeclComponent>> {
        self.decls.get(node_id).map(|r| r.clone())
    }
    pub(crate) fn hooks_snapshot(
        &self,
    ) -> (Vec<Arc<dyn CmpAroundAspect>>, Arc<MonitorBus>) {
        (self.aspects.read().unwrap().clone(), self.monitor.clone())
    }

    // ---------- 组件管理 ----------

    /// addComponent(nodeId, cmpInstance)
    pub fn register<C: NodeComponent>(&self, node_id: impl Into<String>, cmp: C) {
        self.nodes.insert(node_id.into(), Arc::new(cmp));
    }
    pub fn register_arc(&self, node_id: impl Into<String>, cmp: Arc<dyn NodeComponent>) {
        self.nodes.insert(node_id.into(), cmp);
    }
    /// removeComponent
    pub fn unregister(&self, node_id: &str) {
        self.nodes.remove(node_id);
    }
    pub fn contains_node(&self, node_id: &str) -> bool {
        self.nodes.contains_key(node_id)
    }
    pub(crate) fn get_node(&self, node_id: &str) -> Option<Arc<dyn NodeComponent>> {
        self.nodes.get(node_id).map(|r| r.clone())
    }

    // ---------- 链路管理（LiteflowMetaOperator） ----------

    /// LiteFlowChainELBuilder.createChain().setChainId().setEL()
    pub fn add_chain(&self, chain_id: impl Into<String>, el: &str) -> LFResult<()> {
        let ast = parse_el(el)?;
        self.add_chain_el(chain_id, ast)
    }

    /// 以语法树构建链路（平滑加载：先完整构建，再原子替换）
    pub fn add_chain_el(&self, chain_id: impl Into<String>, el: El) -> LFResult<()> {
        let id = chain_id.into();
        let chain = LiteFlowChainELBuilder::new(self.clone()).build_chain(&id, el)?;
        self.chains.insert(id.clone(), Arc::new(chain));
        for h in &self.lifecycle.read().unwrap().chain_build {
            h.post_process_after_chain_build(&id);
        }
        Ok(())
    }

    /// 由已构建的 Chain 直接装配（parser 包用）
    pub fn add_built_chain(&self, chain: Chain) {
        let id = chain.id.clone();
        self.chains.insert(id.clone(), Arc::new(chain));
        for h in &self.lifecycle.read().unwrap().chain_build {
            h.post_process_after_chain_build(&id);
        }
    }

    /// reloadChain：热刷新
    pub fn reload_chain(&self, chain_id: &str, el: &str) -> LFResult<()> {
        if !self.chains.contains_key(chain_id) {
            return Err(LiteflowError::ChainNotFound(chain_id.to_string()));
        }
        let ast = parse_el(el)?;
        self.add_chain_el(chain_id.to_string(), ast)
    }

    pub fn remove_chain(&self, chain_id: &str) {
        self.chains.remove(chain_id);
    }
    pub fn contains_chain(&self, chain_id: &str) -> bool {
        self.chains.contains_key(chain_id)
    }
    pub fn chain_ids(&self) -> Vec<String> {
        self.chains.iter().map(|r| r.key().clone()).collect()
    }
    pub(crate) fn get_chain(&self, chain_id: &str) -> Option<Arc<Chain>> {
        self.chains.get(chain_id).map(|r| r.clone())
    }

    /// 注册脚本节点（对应规则文件 nodes 中的 script 节点；当前支持 language="rhai"）
    pub fn register_script(&self, node_id: impl Into<String>, language: &str, script: &str) -> LFResult<()> {
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
        let id = node_id.into();
        match language {
            "rhai" => {
                let cmp = ScriptComponent::new(&id, kind, script)?;
                self.register_arc(id, Arc::new(cmp));
            }
            #[cfg(feature = "lua")]
            "lua" => {
                let cmp = ScriptComponent::new_lua(&id, kind, script)?;
                self.register_arc(id, Arc::new(cmp));
            }
            other => {
                return Err(LiteflowError::Script {
                    node: id,
                    msg: format!("unsupported script language: {other} (rhai; lua needs feature `lua`)"),
                })
            }
        }
        Ok(())
    }

    /// 构建决策表链路（route + body），对应 setRoute + setEL
    pub fn add_route_chain(
        &self,
        chain_id: impl Into<String>,
        namespace: &str,
        route_el: &str,
        body_el: &str,
    ) -> LFResult<()> {
        let id = chain_id.into();
        let route = parse_el(route_el)?;
        let body = parse_el(body_el)?;
        let chain = LiteFlowChainELBuilder::new(self.clone())
            .build_route_chain(&id, namespace, route, body)?;
        self.chains.insert(id.clone(), Arc::new(chain));
        Ok(())
    }

    /// EL 语法校验（validate）
    pub fn validate_el(el: &str) -> LFResult<()> {
        parse_el(el).map(|_| ())
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
    pub fn executor(&self) -> FlowExecutor {
        FlowExecutor::new(self.clone())
    }
    /// execute2Resp(chainId)
    pub async fn execute(&self, chain_id: &str) -> LiteflowResponse {
        self.executor().execute(chain_id).await
    }
    /// execute2Resp(chainId, requestData)
    pub async fn execute_with_data(&self, chain_id: &str, input: impl Serialize) -> LiteflowResponse {
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
    /// executeRouteChain(namespace, param)
    pub async fn execute_route_chain(
        &self,
        namespace: Option<&str>,
        input: impl Serialize,
    ) -> LFResult<Vec<LiteflowResponse>> {
        self.executor().execute_route_chain(namespace, input).await
    }

    /// execute2Resp(chainId, requestData, timeout, unit)
    pub async fn execute_timeout(
        &self,
        chain_id: &str,
        input: impl Serialize,
        timeout: Duration,
    ) -> LiteflowResponse {
        self.executor().execute_timeout(chain_id, input, timeout).await
    }
}
