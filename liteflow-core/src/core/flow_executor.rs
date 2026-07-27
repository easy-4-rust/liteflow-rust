//! 对应 core.FlowExecutor：执行入口。

use crate::enums::ChainExecuteModeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::flow_bus::FlowBus;
use crate::flow::id::IdGeneratorHolder;
use crate::flow::liteflow_response::LiteflowResponse;
use crate::property::{LiteflowConfig, LiteflowConfigGetter};
use crate::slot::{Ctx, DataBus, Slot};
use crate::thread::ExecutorHelper;
use md5::{Digest, Md5};
use serde::Serialize;
use serde_json::Value;
use std::any::Any;
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// LiteFlow 主执行入口。
///
/// 对应 Java: `com.yomahub.liteflow.core.FlowExecutor`。
#[derive(Clone)]
pub struct FlowExecutor {
    bus: FlowBus,
    liteflow_config: Arc<RwLock<LiteflowConfig>>,
}

impl FlowExecutor {
    /// 使用流程总线和当前全局配置创建执行器。
    ///
    /// 对应 Java 无显式配置的初始化路径；尚未装配时使用
    /// `LiteflowConfigGetter#get` 的默认回退配置。
    pub fn new(bus: FlowBus) -> Self {
        let liteflow_config = LiteflowConfigGetter::get();
        // Java FlowExecutor 无参构造器会调用 DataBus.init()，由当前全局配置确定
        // Slot 池初始容量；Rust 在保存执行器配置前执行同一初始化动作。
        DataBus::init(liteflow_config.get_slot_size());
        Self {
            bus,
            liteflow_config: Arc::new(RwLock::new(liteflow_config)),
        }
    }

    /// 使用指定配置创建执行器并登记到全局配置获取器。
    ///
    /// 参数 `liteflow_config` 对应 Java `FlowExecutor(LiteflowConfig)`。
    #[must_use]
    pub fn new_with_config(bus: FlowBus, liteflow_config: LiteflowConfig) -> Self {
        LiteflowConfigGetter::set_liteflow_config(liteflow_config.clone());
        // 对应 Java FlowExecutor(LiteflowConfig) 构造器末尾的 DataBus.init()。
        DataBus::init(liteflow_config.get_slot_size());
        Self {
            bus,
            liteflow_config: Arc::new(RwLock::new(liteflow_config)),
        }
    }

    /// 返回当前执行器配置快照。对应 Java: `FlowExecutor#getLiteflowConfig`。
    #[must_use]
    pub fn liteflow_config(&self) -> LiteflowConfig {
        self.liteflow_config.read().unwrap().clone()
    }

    /// 返回当前执行器配置快照。
    ///
    /// 这是 Java 公共方法名的严格 snake_case 映射；`liteflow_config` 保留为
    /// Rust 简洁别名。对应 Java: `FlowExecutor#getLiteflowConfig`。
    #[must_use]
    pub fn get_liteflow_config(&self) -> LiteflowConfig {
        self.liteflow_config()
    }

    /// 更新执行器配置并同步全局配置获取器。
    ///
    /// 参数 `liteflow_config` 与 Java `FlowExecutor#setLiteflowConfig` 一致。
    pub fn set_liteflow_config(&self, liteflow_config: LiteflowConfig) {
        *self.liteflow_config.write().unwrap() = liteflow_config.clone();
        LiteflowConfigGetter::set_liteflow_config(liteflow_config);
    }

    /// execute2Resp(chainId)
    pub async fn execute(&self, chain_id: &str) -> LiteflowResponse {
        self.execute_with(
            chain_id,
            Value::Null,
            Vec::<(String, Arc<dyn Any + Send + Sync>)>::new(),
        )
        .await
    }

    /// execute2Resp(chainId, requestData)
    pub async fn execute_with_data(
        &self,
        chain_id: &str,
        input: impl Serialize,
    ) -> LiteflowResponse {
        let v = serde_json::to_value(input).unwrap_or(Value::Null);
        self.execute_with(
            chain_id,
            v,
            Vec::<(String, Arc<dyn Any + Send + Sync>)>::new(),
        )
        .await
    }

    /// execute2Resp(chainId, requestData, contextBeanArray)
    pub async fn execute_with(
        &self,
        chain_id: &str,
        input: Value,
        beans: Vec<(String, Arc<dyn Any + Send + Sync>)>,
    ) -> LiteflowResponse {
        self.execute_with_option(
            chain_id,
            input,
            crate::core::execute_option::ExecuteOption {
                context_beans: beans,
                ..Default::default()
            },
        )
        .await
    }

    /// 使用执行选项执行链路并返回响应。
    ///
    /// - `chain_id`: Java `chainId`，目标链路标识。
    /// - `param`: Java `param`，本次链路输入。
    /// - `option`: Java `option`；`None` 等价于 `ExecuteOption::of()`。
    ///
    /// 对应 Java: `FlowExecutor#execute2Resp(String, Object, ExecuteOption)`。
    pub async fn execute2_resp(
        &self,
        chain_id: &str,
        param: Value,
        option: Option<crate::core::execute_option::ExecuteOption>,
    ) -> LiteflowResponse {
        self.execute_with_option(chain_id, param, option.unwrap_or_default())
            .await
    }

    /// 异步提交一次链路执行并返回 Tokio 任务句柄。
    ///
    /// 任务由 `LiteFlowDefaultMainExecutorBuilder` 构建的有界主执行器控制并发；
    /// 对应 Java `FlowExecutor#execute2Future(String, Object, ExecuteOption)`。
    pub fn execute_future_with_option(
        &self,
        chain_id: impl Into<String>,
        input: Value,
        option: crate::core::execute_option::ExecuteOption,
    ) -> LFResult<tokio::task::JoinHandle<LiteflowResponse>> {
        self.execute_future_with_executor(chain_id, input, option, None)
    }

    /// 异步提交链路执行。
    ///
    /// - `chain_id`: Java `chainId`。
    /// - `param`: Java `param`。
    /// - `option`: Java `option`；`None` 使用默认执行选项。
    ///
    /// 对应 Java: `FlowExecutor#execute2Future(String, Object, ExecuteOption)`。
    pub fn execute2_future(
        &self,
        chain_id: impl Into<String>,
        param: Value,
        option: Option<crate::core::execute_option::ExecuteOption>,
    ) -> LFResult<tokio::task::JoinHandle<LiteflowResponse>> {
        self.execute_future_with_option(chain_id, param, option.unwrap_or_default())
    }

    /// 使用指定执行器构建器提交异步链路。
    ///
    /// Java 通过 `mainExecutorClass` 配置选择构建器；Rust 额外暴露该参数，便于
    /// 独立运行时在没有 Vernal 容器时显式选择已注册构建器。
    pub fn execute_future_with_executor(
        &self,
        chain_id: impl Into<String>,
        input: Value,
        option: crate::core::execute_option::ExecuteOption,
        executor_class: Option<&str>,
    ) -> LFResult<tokio::task::JoinHandle<LiteflowResponse>> {
        let executor_service =
            ExecutorHelper::load_instance().build_main_executor(executor_class)?;
        let executor = self.clone();
        let chain_id = chain_id.into();
        let failure_chain_id = chain_id.clone();
        let failure_input = input.clone();
        let failure_request_id = option
            .request_id
            .clone()
            .filter(|request_id| !request_id.trim().is_empty())
            .unwrap_or_else(IdGeneratorHolder::generate);
        Ok(tokio::spawn(async move {
            match executor_service
                .execute(executor.execute_with_option(&chain_id, input, option))
                .await
            {
                Ok(response) => response,
                Err(error) => {
                    let slot = Arc::new(Slot::new(
                        failure_request_id,
                        failure_chain_id,
                        failure_input,
                    ));
                    LiteflowResponse::new(slot, false, error.to_string(), Some(error.to_string()))
                }
            }
        }))
    }

    /// execute2RespWithRid(chainId, requestData, requestId, contextBeans)（2.16：
    /// 组件内 invoke2Resp 语义——以同一 requestId 执行子链）
    pub async fn execute_with_rid(
        &self,
        chain_id: &str,
        input: Value,
        request_id: impl Into<String>,
        beans: Vec<(String, Arc<dyn Any + Send + Sync>)>,
    ) -> LiteflowResponse {
        self.execute_with_option(
            chain_id,
            input,
            crate::core::execute_option::ExecuteOption {
                request_id: Some(request_id.into()),
                context_beans: beans,
                ..Default::default()
            },
        )
        .await
    }

    /// 使用指定请求 ID 执行链路。
    ///
    /// - `chain_id`: Java `chainId`。
    /// - `param`: Java `param`。
    /// - `request_id`: Java `requestId`。
    /// - `context_beans`: Java `contextBeanArray` 的 Rust 具名对象映射。
    ///
    /// 对应 Java: `FlowExecutor#execute2RespWithRid`。
    pub async fn execute2_resp_with_rid(
        &self,
        chain_id: &str,
        param: Value,
        request_id: impl Into<String>,
        context_beans: Vec<(String, Arc<dyn Any + Send + Sync>)>,
    ) -> LiteflowResponse {
        self.execute_with_rid(chain_id, param, request_id, context_beans)
            .await
    }

    /// 使用指定请求 ID 异步提交链路执行。
    ///
    /// 对应 Java: `FlowExecutor#execute2FutureWithRid`。
    pub fn execute2_future_with_rid(
        &self,
        chain_id: impl Into<String>,
        param: Value,
        request_id: impl Into<String>,
        context_beans: Vec<(String, Arc<dyn Any + Send + Sync>)>,
    ) -> LFResult<tokio::task::JoinHandle<LiteflowResponse>> {
        self.execute_future_with_option(
            chain_id,
            param,
            crate::core::execute_option::ExecuteOption {
                request_id: Some(request_id.into()),
                context_beans,
                ..Default::default()
            },
        )
    }

    /// 执行 Flow/Chain 的 before 生命周期。
    ///
    /// 对应 Java `FlowExecutor#doExecute` 进入链路前的生命周期处理。先克隆钩子再
    /// await，避免持有 `std::sync::RwLock` guard 跨异步挂起点。
    async fn run_before_hooks(&self, chain_id: &str) {
        let (flow_hooks, chain_hooks) = {
            let hooks = self.bus.lifecycle.read().unwrap();
            (hooks.flow_execute.clone(), hooks.chain_execute.clone())
        };
        for hook in flow_hooks {
            hook.post_process_before_flow_execute(chain_id).await;
        }
        for hook in chain_hooks {
            hook.post_process_before_chain_execute(chain_id).await;
        }
    }

    /// 执行 Chain/Flow 的 after 生命周期。
    ///
    /// 对应 Java `FlowExecutor#doExecute` 的 finally 块：成功、组件异常、找不到
    /// Chain 与超时都必须进入 after 生命周期。
    async fn run_after_hooks(&self, chain_id: &str) {
        let (chain_hooks, flow_hooks) = {
            let hooks = self.bus.lifecycle.read().unwrap();
            (hooks.chain_execute.clone(), hooks.flow_execute.clone())
        };
        for hook in chain_hooks {
            hook.post_process_after_chain_execute(chain_id).await;
        }
        for hook in flow_hooks {
            hook.post_process_after_flow_execute(chain_id).await;
        }
    }

    /// 在已经创建好的 Slot 上执行 Chain 主体。
    ///
    /// 普通异常会先写入 Slot，再按执行记录逆序补偿；`ChainEnd` 是正常终止，
    /// 与 Java 一样不触发 rollback。
    async fn execute_chain_on_slot(&self, chain_id: &str, slot: Arc<Slot>) -> LiteflowResponse {
        let ctx = Ctx::new(slot.clone());
        let chain = match self.bus.get_chain(chain_id) {
            Some(chain) => chain,
            None => {
                let error = LiteflowError::ChainNotFound(chain_id.to_string());
                ctx.set_exception(&error.to_string());
                return LiteflowResponse::new_main_response(slot);
            }
        };

        match chain.execute_mode(&ctx, ChainExecuteModeEnum::Body).await {
            Ok(_) | Err(LiteflowError::ChainEnd(_)) => {
                // retry 成功、ignoreError 或 continueOnError 可能在中途写入过异常；
                // 最终链路成功时必须清除主异常，否则 Java newMainResponse(slot)
                // 会把已经恢复的执行错误地判为失败。
                slot.remove_exception();
                LiteflowResponse::new_main_response(slot)
            }
            Err(error) => {
                ctx.set_exception(&error.to_string());
                ctx.rollback().await;
                LiteflowResponse::new_main_response(slot)
            }
        }
    }

    /// execute2Resp(chainId, requestData, ExecuteOption)（2.16 新增入口：
    /// requestId / conversationId / eventListener / contextBeans）
    pub async fn execute_with_option(
        &self,
        chain_id: &str,
        input: Value,
        option: crate::core::execute_option::ExecuteOption,
    ) -> LiteflowResponse {
        let request_id = option
            .request_id
            .clone()
            .filter(|request_id| !request_id.trim().is_empty())
            .unwrap_or_else(IdGeneratorHolder::generate);
        let mut slot = Slot::new(request_id, chain_id, input);
        if let Some(conversation_id) = option.resolve_conversation_id() {
            slot.set_conversation_id(conversation_id);
        }
        // Slot 进入 DataBus 后由租约托管；即使异步执行被取消，Drop 也会归还索引。
        let slot_lease = DataBus::lease_slot(Arc::new(slot));
        let slot = slot_lease.slot();
        for (name, context_bean_factory) in option.context_bean_classes {
            // Java 直到 DataBus.offerSlotByClass 才反射构造 Bean；Rust 在相同阶段
            // 调用强类型工厂，确保每次执行获得独立上下文实例。
            slot.insert_context_bean(name, context_bean_factory());
        }
        for (name, bean) in option.context_beans {
            slot.insert_context_bean(name, bean);
        }
        let ctx = Ctx::new(slot.clone());
        if let Some(l) = &option.event_listener {
            crate::flow::flow_event_publisher::FlowEventPublisher::set_listener(&ctx, l.clone());
        }

        self.run_before_hooks(chain_id).await;
        let response = self.execute_chain_on_slot(chain_id, slot).await;
        crate::flow::flow_event_publisher::FlowEventPublisher::remove_listener(&ctx);
        self.run_after_hooks(chain_id).await;
        response
    }

    /// execute2RespWithEL(elStr)（2.16 新增：直接执行 EL 表达式）
    pub async fn execute_with_el(&self, el_str: &str) -> LiteflowResponse {
        self.execute_with_el_full(el_str, Value::Null, None).await
    }

    /// execute2RespWithEL(elStr, param)
    pub async fn execute_with_el_data(
        &self,
        el_str: &str,
        input: impl Serialize,
    ) -> LiteflowResponse {
        let v = serde_json::to_value(input).unwrap_or(Value::Null);
        self.execute_with_el_full(el_str, v, None).await
    }

    /// execute2RespWithEL(elStr, param, requestId)
    pub async fn execute_with_el_full(
        &self,
        el_str: &str,
        input: Value,
        request_id: Option<String>,
    ) -> LiteflowResponse {
        self.execute2_resp_with_el(
            el_str,
            input,
            request_id,
            Vec::<(String, Arc<dyn Any + Send + Sync>)>::new(),
        )
        .await
    }

    /// 直接执行 EL，并传入请求 ID 与上下文 Bean。
    ///
    /// - `el_str`: Java `elStr`，待执行的 EL 表达式。
    /// - `param`: Java `param`，链路输入。
    /// - `request_id`: Java `requestId`；`None` 时自动生成。
    /// - `context_beans`: Java `contextBeanArray` 的 Rust 具名对象映射。
    ///
    /// 对应 Java: `FlowExecutor#execute2RespWithEL`。
    pub async fn execute2_resp_with_el(
        &self,
        el_str: &str,
        param: Value,
        request_id: Option<String>,
        context_beans: Vec<(String, Arc<dyn Any + Send + Sync>)>,
    ) -> LiteflowResponse {
        // 规范化 EL（对应 ElRegexUtil.normalize：单引号→双引号、去空白、末尾保留一个分号）
        let normalized = crate::util::el_regex_util::normalize_el(el_str);
        let el_md5 = format!("{:x}", Md5::digest(normalized.as_bytes()));

        let chain_id = match self.bus.get_chain_id_by_el_md5(&el_md5) {
            Some(id) => id,
            None => {
                // 匿名链路：UUID 语义的唯一 chainId
                let id = format!(
                    "anon_{:x}",
                    Md5::digest(
                        format!("{}-{}", normalized, IdGeneratorHolder::generate()).as_bytes(),
                    )
                );
                if let Err(e) = self
                    .bus
                    .add_chain_anonymous(&id, &normalized, el_md5.clone())
                {
                    let slot = Arc::new(Slot::new(IdGeneratorHolder::generate(), &id, param));
                    return LiteflowResponse::new(slot, false, e.to_string(), Some(e.to_string()));
                }
                id
            }
        };
        self.execute_with_option(
            &chain_id,
            param,
            crate::core::execute_option::ExecuteOption {
                request_id,
                context_beans,
                ..Default::default()
            },
        )
        .await
    }

    /// executeRouteChain(namespace, param, contextBeans)：
    /// 并行求值 namespace 下所有决策表链路的 route EL，
    /// 命中的链路并行执行 body，返回各链路的响应（对应 doExecuteWithRoute）。
    pub async fn execute_route_chain(
        &self,
        namespace: Option<&str>,
        input: impl Serialize,
    ) -> crate::exception::LFResult<Vec<LiteflowResponse>> {
        self.execute_route_chain_with_rid(namespace, input, IdGeneratorHolder::generate())
            .await
    }

    /// executeRouteChainWithRid(namespace, param, requestId)。
    ///
    /// Java 在一次路由决策中先生成 `finalRequestId`，随后所有 route EL 与命中的
    /// body Chain 都复用该值。Rust 每个执行仍使用独立 Slot，但链路关联 ID 保持一致。
    pub async fn execute_route_chain_with_rid(
        &self,
        namespace: Option<&str>,
        input: impl Serialize,
        request_id: impl Into<String>,
    ) -> crate::exception::LFResult<Vec<LiteflowResponse>> {
        let namespace = namespace.unwrap_or(crate::flow::element::chain::DEFAULT_NAMESPACE);
        let v = serde_json::to_value(input).unwrap_or(Value::Null);
        let request_id = {
            let request_id = request_id.into();
            if request_id.trim().is_empty() {
                IdGeneratorHolder::generate()
            } else {
                request_id
            }
        };

        let route_chains: Vec<Arc<crate::flow::element::chain::Chain>> = self
            .bus
            .chain_ids()
            .into_iter()
            .filter_map(|id| self.bus.get_chain(&id))
            .filter(|c| c.get_namespace() == namespace && c.get_route_item().is_some())
            .collect();
        if route_chains.is_empty() {
            return Err(LiteflowError::RouteChainNotFound(namespace.to_string()));
        }

        // 并行求 route EL（每条链路独立 slot，同 requestId 语义）
        let mut set: tokio::task::JoinSet<(Arc<crate::flow::element::chain::Chain>, bool)> =
            tokio::task::JoinSet::new();
        for chain in route_chains {
            let v = v.clone();
            let request_id = request_id.clone();
            let executor = FlowExecutor::new(self.bus.clone());
            set.spawn(async move {
                let slot_lease =
                    DataBus::lease_slot(Arc::new(Slot::new(request_id, chain.id.clone(), v)));
                let slot = slot_lease.slot();
                let ctx = Ctx::new(slot.clone());
                executor.run_before_hooks(&chain.id).await;
                let matched = match chain.execute_mode(&ctx, ChainExecuteModeEnum::Route).await {
                    Ok(Value::Bool(matched)) => matched,
                    Ok(_) => false,
                    Err(error) => {
                        ctx.set_exception(&error.to_string());
                        ctx.rollback().await;
                        false
                    }
                };
                executor.run_after_hooks(&chain.id).await;
                (chain, matched)
            });
        }
        let mut matched = Vec::new();
        while let Some(Ok((chain, ok))) = set.join_next().await {
            if ok {
                matched.push(chain);
            }
        }
        if matched.is_empty() {
            return Err(LiteflowError::NoMatchedRouteChain(
                "there is no matched route chain".to_string(),
            ));
        }

        // 命中的链路并行执行 body
        let mut set: tokio::task::JoinSet<(usize, LiteflowResponse)> = tokio::task::JoinSet::new();
        for (index, chain) in matched.into_iter().enumerate() {
            let v = v.clone();
            let request_id = request_id.clone();
            let executor = FlowExecutor::new(self.bus.clone());
            set.spawn(async move {
                let response = executor
                    .execute_with_option(
                        &chain.id,
                        v,
                        crate::core::execute_option::ExecuteOption::of().request_id(request_id),
                    )
                    .await;
                (index, response)
            });
        }
        let mut responses = Vec::new();
        while let Some(Ok(response)) = set.join_next().await {
            responses.push(response);
        }
        responses.sort_by_key(|(index, _)| *index);
        Ok(responses
            .into_iter()
            .map(|(_, response)| response)
            .collect())
    }

    /// execute2Resp(chainId, requestData, timeout, unit)
    pub async fn execute_timeout(
        &self,
        chain_id: &str,
        input: impl Serialize,
        timeout: Duration,
    ) -> LiteflowResponse {
        let input = serde_json::to_value(input).unwrap_or(Value::Null);
        let request_id = IdGeneratorHolder::generate();
        let slot_lease = DataBus::lease_slot(Arc::new(Slot::new(request_id, chain_id, input)));
        let slot = slot_lease.slot();
        let ctx = Ctx::new(slot.clone());

        self.run_before_hooks(chain_id).await;
        let response =
            match tokio::time::timeout(timeout, self.execute_chain_on_slot(chain_id, slot.clone()))
                .await
            {
                Ok(response) => response,
                Err(_) => {
                    let message = "chain execute timeout".to_string();
                    ctx.set_exception(&message);
                    ctx.rollback().await;
                    LiteflowResponse::new(slot, false, message.clone(), Some(message))
                }
            };
        self.run_after_hooks(chain_id).await;
        response
    }
}
