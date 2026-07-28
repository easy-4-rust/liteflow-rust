//! 对应 core.FlowExecutor：执行入口。

use crate::core::FlowInitHook;
use crate::el::NodeRef;
use crate::enums::{ChainExecuteModeEnum, FlowParserTypeEnum};
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::node::Node;
use crate::flow::flow_bus::FlowBus;
use crate::flow::id::IdGeneratorHolder;
use crate::flow::liteflow_response::LiteflowResponse;
use crate::lifecycle::ChainCacheLifeCycle;
use crate::monitor::MonitorFile;
use crate::parser::FlowParserProvider;
use crate::property::{LiteflowConfig, LiteflowConfigGetter};
use crate::slot::{Ctx, DataBus, Frame, Slot};
use crate::spi::ContextCmpInitHolder;
use crate::thread::ExecutorHelper;
use md5::{Digest, Md5};
use serde::Serialize;
use serde_json::Value;
use std::any::Any;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// LiteFlow 主执行入口。
///
/// 对应 Java: `com.yomahub.liteflow.core.FlowExecutor`。
#[derive(Clone)]
pub struct FlowExecutor {
    bus: FlowBus,
    liteflow_config: Arc<RwLock<LiteflowConfig>>,
    parser_provider: FlowParserProvider,
    monitor_file: Arc<RwLock<Option<Arc<MonitorFile>>>>,
    start_up_phase: Arc<AtomicBool>,
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
            parser_provider: FlowParserProvider::new(bus.clone()),
            bus,
            liteflow_config: Arc::new(RwLock::new(liteflow_config)),
            monitor_file: Arc::new(RwLock::new(None)),
            start_up_phase: Arc::new(AtomicBool::new(false)),
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
            parser_provider: FlowParserProvider::new(bus.clone()),
            bus,
            liteflow_config: Arc::new(RwLock::new(liteflow_config)),
            monitor_file: Arc::new(RwLock::new(None)),
            start_up_phase: Arc::new(AtomicBool::new(false)),
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

    /// 注册 Rust 自定义规则内容源，供 `init` 按 Java 自定义 Parser 前缀加载。
    ///
    /// - `class_name`：Java Parser 实现类全名对应的稳定注册名。
    /// - `parser_type`：规则内容格式。
    /// - `content_provider`：返回真实规则文本的内容提供器。
    ///
    /// Rust 使用显式注册替代 `Class.forName`。对应 Java:
    /// `FlowParserProvider#lookup` 的自定义 Parser 分支。
    pub fn register_class_parser(
        &self,
        class_name: impl Into<String>,
        parser_type: FlowParserTypeEnum,
        content_provider: Arc<dyn Fn() -> LFResult<String> + Send + Sync>,
    ) {
        self.parser_provider
            .register_class_parser(class_name, parser_type, content_provider);
    }

    /// 初始化组件、请求 ID、规则解析、缓存生命周期和启动钩子。
    ///
    /// - `is_start`：是否为首次启动；重载规则时传入 `false`。
    /// - 返回：全部初始化动作成功时返回 `Ok(())`，否则返回具体 LiteFlow 错误。
    ///
    /// 启动阶段使用析构守卫复位，因此规则解析或监听器创建失败时不会把执行器
    /// 永久留在启动状态。对应 Java: `FlowExecutor#init(boolean)`。
    pub fn init(&self, is_start: bool) -> LFResult<()> {
        if is_start {
            self.start_up_phase.store(true, Ordering::Release);
        }
        let _start_up_phase_guard =
            StartUpPhaseGuard::new(Arc::clone(&self.start_up_phase), is_start);
        let liteflow_config = self.get_liteflow_config();

        // 容器组件初始化在首次启动和规则重载时都执行，与 Java SPI 调用位置一致。
        ContextCmpInitHolder::load_context_cmp_init().init_cmp();
        if is_start {
            IdGeneratorHolder::init()?;
        }

        if is_start && liteflow_config.get_chain_cache_enabled() {
            let capacity = liteflow_config.get_chain_cache_capacity();
            let cleaner = self.bus.chain_cache_cleaner();
            ChainCacheLifeCycle::init_if_absent(capacity, cleaner);
            let cache = ChainCacheLifeCycle::get_life_cycle();
            if let Some(cache) = cache {
                self.bus.register_chain_execute_hook(cache);
            }
        }

        let Some(rule_source) = liteflow_config
            .get_rule_source()
            .map(str::trim)
            .filter(|rule_source| !rule_source.is_empty())
        else {
            // Java 允许完全通过代码动态构建 Chain；没有规则源不属于初始化失败。
            return Ok(());
        };
        let rule_paths = split_rule_source(rule_source);
        self.parse_rule_paths(&rule_paths, liteflow_config.is_support_multiple_type())?;

        if is_start {
            FlowInitHook::execute_hook();
        }

        if is_start && liteflow_config.get_enable_monitor_file() {
            let monitor_paths: Vec<&str> = rule_paths
                .iter()
                .map(String::as_str)
                .filter(|path| !is_custom_parser_path(path) && Path::new(path).exists())
                .collect();
            if !monitor_paths.is_empty() {
                // MonitorFile 内部使用 Tokio 后台任务；显式验证运行时可用性，将原本
                // 的 tokio::spawn panic 转换为可诊断的 LiteFlow 初始化错误。
                tokio::runtime::Handle::try_current().map_err(|error| {
                    LiteflowError::MonitorFileInitError(format!(
                        "monitor file requires an active Tokio runtime: {error}"
                    ))
                })?;
                let monitor_file = MonitorFile::get_instance(self.bus.clone());
                monitor_file.add_monitor_file_paths(monitor_paths)?;
                monitor_file.create(Duration::from_secs(1))?;
                *self.monitor_file.write().unwrap() = Some(monitor_file);
            }
        }
        Ok(())
    }

    /// 重新读取当前配置中的全部规则源。
    ///
    /// - 返回：规则重新解析成功时返回 `Ok(())`；旧规则在解析失败时仍由解析器的
    ///   平滑装载语义保留。
    ///
    /// 重载不会重复初始化请求 ID、启动钩子或文件监听任务。对应 Java:
    /// `FlowExecutor#reloadRule()`。
    pub fn reload_rule(&self) -> LFResult<()> {
        self.init(false)
    }

    /// 在指定 Slot 中执行一个已经注册的节点。
    ///
    /// - `node_id`：Java `nodeId`，目标节点标识。
    /// - `slot_index`：Java `slotIndex`，必须仍由 `DataBus` 持有。
    /// - 返回：节点真实执行结果；节点或 Slot 不存在以及组件执行失败时返回错误。
    ///
    /// 该入口经 `NodeExecutor` 完成访问判断、重试、回调与步骤记录，而不是绕过
    /// 生命周期直接调用组件。对应 Java: `FlowExecutor#invoke(String, Integer)`。
    #[deprecated(note = "仅用于兼容 Java FlowExecutor#invoke；优先执行 Chain")]
    pub async fn invoke(&self, node_id: &str, slot_index: usize) -> LFResult<Value> {
        let slot = DataBus::get_slot(slot_index).ok_or_else(|| {
            LiteflowError::DataNotFound(format!("slot index does not exist: {slot_index}"))
        })?;
        let component = self
            .bus
            .get_node(node_id)
            .ok_or_else(|| LiteflowError::NodeNotFound(node_id.to_string()))?;
        let mut node = Node::new(NodeRef::new(node_id), component);
        node.set_curr_chain_id(slot.chain_id.clone());
        node.execute(&Ctx::new(slot), &Frame::root()).await
    }

    /// 返回共享的启动阶段状态。
    ///
    /// - 返回：与执行器克隆实例共享的 `AtomicBool`；`init(true)` 执行期间为
    ///   `true`，正常返回和错误返回后均为 `false`。
    ///
    /// 对应 Java: `FlowExecutor#getStartUpPhase()`。
    #[must_use]
    pub fn get_start_up_phase(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.start_up_phase)
    }

    fn parse_rule_paths(&self, rule_paths: &[String], support_multiple_type: bool) -> LFResult<()> {
        if rule_paths.is_empty() {
            return Ok(());
        }
        if support_multiple_type {
            for rule_path in rule_paths {
                let parser = self.parser_provider.lookup(rule_path)?;
                parser.parse_main(parser_arguments(rule_path))?;
            }
            return Ok(());
        }

        let expected_identity = parser_identity(&rule_paths[0]);
        if rule_paths
            .iter()
            .skip(1)
            .any(|rule_path| parser_identity(rule_path) != expected_identity)
        {
            return Err(LiteflowError::MultipleParsers(
                "multiple parser types found while supportMultipleType is false".to_string(),
            ));
        }
        let parser = self.parser_provider.lookup(&rule_paths[0])?;
        if is_custom_parser_path(&rule_paths[0]) {
            parser.parse_main(&[])?;
        } else {
            parser.parse_main(rule_paths)?;
        }
        Ok(())
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

    /// 绑定 Chain 生命周期快照并执行 Flow 的 before 生命周期。
    ///
    /// Chain 生命周期由 `Chain#execute` 在主链和子链边界调用；此处只负责把
    /// 当前 FlowBus 的快照绑定到 Slot，并执行 Java `FlowExecutor#doExecute`
    /// 的 before 回调。先克隆钩子再 await，避免持锁跨异步挂起点。
    async fn run_before_hooks(&self, chain_id: &str, slot: &Arc<Slot>) {
        let (flow_hooks, chain_hooks) = {
            let hooks = self.bus.lifecycle.read().unwrap();
            (hooks.flow_execute.clone(), hooks.chain_execute.clone())
        };
        slot.set_chain_execute_life_cycles(chain_hooks);
        for hook in flow_hooks {
            hook.post_process_before_flow_execute(chain_id, slot).await;
        }
    }

    /// 执行 Flow 的 after 生命周期。
    ///
    /// 对应 Java `FlowExecutor#doExecute` 的 finally 块：成功、组件异常、找不到
    /// Chain 与超时都必须进入 after 生命周期。
    async fn run_after_hooks(&self, chain_id: &str, slot: &Arc<Slot>) {
        let flow_hooks = self.bus.lifecycle.read().unwrap().flow_execute.clone();
        for hook in flow_hooks {
            hook.post_process_after_flow_execute(chain_id, slot).await;
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
                let error_code = error.get_code().map(ToOwned::to_owned);
                ctx.set_exception(&error.to_string());
                ctx.rollback().await;
                let mut response = LiteflowResponse::new_main_response(slot);
                response.set_code(error_code);
                response
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

        self.run_before_hooks(chain_id, &slot).await;
        let response = self.execute_chain_on_slot(chain_id, slot.clone()).await;
        crate::flow::flow_event_publisher::FlowEventPublisher::remove_listener(&ctx);
        self.run_after_hooks(chain_id, &slot).await;
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
        let normalized = crate::util::el_regex_util::ElRegexUtil::normalize(el_str);
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
                executor.run_before_hooks(&chain.id, &slot).await;
                let matched = match chain.execute_mode(&ctx, ChainExecuteModeEnum::Route).await {
                    Ok(Value::Bool(matched)) => matched,
                    Ok(_) => false,
                    Err(error) => {
                        ctx.set_exception(&error.to_string());
                        ctx.rollback().await;
                        false
                    }
                };
                executor.run_after_hooks(&chain.id, &slot).await;
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

        self.run_before_hooks(chain_id, &slot).await;
        let response =
            match tokio::time::timeout(timeout, self.execute_chain_on_slot(chain_id, slot.clone()))
                .await
            {
                Ok(response) => response,
                Err(_) => {
                    let message = "chain execute timeout".to_string();
                    ctx.set_exception(&message);
                    ctx.rollback().await;
                    LiteflowResponse::new(slot.clone(), false, message.clone(), Some(message))
                }
            };
        self.run_after_hooks(chain_id, &slot).await;
        response
    }
}

/// `FlowExecutor` 的内部伴随守卫，在 `init(true)` 的所有返回路径复位启动状态。
///
/// 对应 Java `FlowExecutor#startUpPhase` 在初始化结束时恢复 `false` 的语义。
struct StartUpPhaseGuard {
    start_up_phase: Arc<AtomicBool>,
    active: bool,
}

impl StartUpPhaseGuard {
    fn new(start_up_phase: Arc<AtomicBool>, active: bool) -> Self {
        Self {
            start_up_phase,
            active,
        }
    }
}

impl Drop for StartUpPhaseGuard {
    fn drop(&mut self) {
        if self.active {
            self.start_up_phase.store(false, Ordering::Release);
        }
    }
}

fn split_rule_source(rule_source: &str) -> Vec<String> {
    rule_source
        .replace(' ', "")
        .split([',', ';'])
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn parser_arguments(rule_path: &String) -> &[String] {
    if is_custom_parser_path(rule_path) {
        &[]
    } else {
        std::slice::from_ref(rule_path)
    }
}

fn is_custom_parser_path(rule_path: &str) -> bool {
    let lower = rule_path.to_ascii_lowercase();
    rule_path
        .split_once(':')
        .is_some_and(|(prefix, _)| FlowParserTypeEnum::get_enum_by_type(prefix).is_some())
        || (!lower.ends_with(".xml") && !lower.ends_with(".json") && !lower.ends_with(".yml"))
}

fn parser_identity(rule_path: &str) -> Option<(FlowParserTypeEnum, bool)> {
    if let Some((prefix, _)) = rule_path.split_once(':') {
        return FlowParserTypeEnum::get_enum_by_type(prefix).map(|parser_type| (parser_type, true));
    }
    let lower = rule_path.to_ascii_lowercase();
    let parser_type = if lower.ends_with(".el.xml") {
        FlowParserTypeEnum::TypeElXml
    } else if lower.ends_with(".el.json") {
        FlowParserTypeEnum::TypeElJson
    } else if lower.ends_with(".el.yml") {
        FlowParserTypeEnum::TypeElYml
    } else if lower.ends_with(".xml") {
        FlowParserTypeEnum::TypeXml
    } else if lower.ends_with(".json") {
        FlowParserTypeEnum::TypeJson
    } else if lower.ends_with(".yml") {
        FlowParserTypeEnum::TypeYml
    } else {
        return None;
    };
    Some((parser_type, false))
}
