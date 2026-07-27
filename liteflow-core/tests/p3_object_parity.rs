//! Java v2.16.0 首批缺失对象的真实语义门禁。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use liteflow_core::builder::el::LiteFlowChainELBuilder;
use liteflow_core::flow::element::chain::Chain;
use liteflow_core::flow::element::condition::then_condition::ThenCondition;
use liteflow_core::flow::element::executable::Executable;
use liteflow_core::flow::element::node::Node;
use liteflow_core::flow::instance_id::{
    BaseNodeInstanceIdManageSpi, DefaultNodeInstanceIdManageSpiImpl, NodeInstanceIdManageSpi,
    NodeInstanceIdManageSpiHolder,
};
use liteflow_core::script::{RhaiScriptExecutor, ScriptExecutor};
use liteflow_core::{
    AbstractCondition, ChainConstant, ChainExecuteModeEnum, CmpContext, ComponentInitializer,
    Condition, ConditionTypeEnum, DataBus, ExecuteableTypeEnum, FlowBus, FlowExecutor,
    FlowExecutorHolder, FlowInitHook, FlowParserTypeEnum, LiteflowConfig, LiteflowConfigGetter,
    LiteflowError, LocalDefaultFlowConstant, LoopFutureObj, NodeBooleanComponent, NodeComponent,
    NodeForComponent, NodeIteratorComponent, NodeRef, NodeSwitchComponent, NodeTypeEnum,
    ParallelStrategyEnum, ParallelStrategyHelper, ParseModeEnum,
    PostProcessScriptEngineInitLifeCycle, Slot, cmp,
};
use serde_json::{Value, json};

struct DirectRollbackComponent {
    calls: Arc<AtomicUsize>,
}

struct PrefixInstanceIdSpi;

struct DataBusAwareComponent {
    seen_slot_index: Arc<Mutex<Option<usize>>>,
}

struct CancellationProbeComponent {
    seen_slot_index: Arc<Mutex<Option<usize>>>,
    started: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
}

struct ScriptEngineInitHook {
    languages: Arc<Mutex<Vec<String>>>,
}

impl PostProcessScriptEngineInitLifeCycle for ScriptEngineInitHook {
    fn post_process_after_script_engine_init(&self, language: &str) {
        self.languages.lock().unwrap().push(language.to_string());
    }
}

impl liteflow_core::LifeCycle for ScriptEngineInitHook {
    fn register_life_cycle(
        self: Arc<Self>,
        life_cycle_holder: &mut liteflow_core::LifeCycleHolder,
    ) {
        life_cycle_holder.script_engine_init.push(self);
    }
}

impl NodeInstanceIdManageSpi for PrefixInstanceIdSpi {
    fn gen_instance_id(&self, chain_id: &str, node_id: &str, occurrence: usize) -> String {
        format!("{chain_id}:{node_id}:{occurrence}")
    }
}

#[liteflow_core::async_trait]
impl NodeComponent for DirectRollbackComponent {
    async fn process(&self, _ctx: &liteflow_core::CmpContext) -> Result<Value, LiteflowError> {
        Ok(Value::Null)
    }

    fn is_rollback(&self) -> bool {
        true
    }

    async fn rollback(&self, _ctx: &liteflow_core::CmpContext) -> Result<(), LiteflowError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn name(&self) -> &str {
        "direct-rollback"
    }
}

#[liteflow_core::async_trait]
impl NodeComponent for DataBusAwareComponent {
    async fn process(&self, ctx: &liteflow_core::CmpContext) -> Result<Value, LiteflowError> {
        let slot_index = ctx
            .slot_index()
            .ok_or_else(|| LiteflowError::WhenExecute("slot not registered in DataBus".into()))?;
        let registered = DataBus::get_slot(slot_index)
            .ok_or_else(|| LiteflowError::WhenExecute("registered slot not found".into()))?;
        if !Arc::ptr_eq(&registered, &ctx.inner) {
            return Err(LiteflowError::WhenExecute(
                "DataBus returned a different slot".into(),
            ));
        }
        *self.seen_slot_index.lock().unwrap() = Some(slot_index);
        Ok(Value::Null)
    }
}

#[liteflow_core::async_trait]
impl NodeComponent for CancellationProbeComponent {
    async fn process(&self, ctx: &liteflow_core::CmpContext) -> Result<Value, LiteflowError> {
        *self.seen_slot_index.lock().unwrap() = ctx.slot_index();
        if let Some(started) = self.started.lock().unwrap().take() {
            let _ = started.send(());
        }
        std::future::pending::<()>().await;
        Ok(Value::Null)
    }
}

#[test]
fn java_enum_names_and_parser_codes_are_preserved() {
    assert_eq!(
        serde_json::to_string(&ChainExecuteModeEnum::Route).unwrap(),
        "\"ROUTE\""
    );
    assert_eq!(
        serde_json::to_string(&ExecuteableTypeEnum::Condition).unwrap(),
        "\"CONDITION\""
    );
    assert_eq!(
        serde_json::to_string(&ParseModeEnum::ParseOneOnFirstExec).unwrap(),
        "\"PARSE_ONE_ON_FIRST_EXEC\""
    );
    assert_eq!(FlowParserTypeEnum::TypeElXml.get_type(), "el_xml");
    assert_eq!(
        FlowParserTypeEnum::get_enum_by_type("json"),
        Some(FlowParserTypeEnum::TypeJson)
    );
    assert_eq!(ConditionTypeEnum::AndOr.get_type(), "and_or_opt");
    assert_eq!(ConditionTypeEnum::Not.get_type(), "not_opt");
    assert_eq!(
        ConditionTypeEnum::get_enum_by_code("abstract"),
        Some(ConditionTypeEnum::Abstract)
    );
}

#[test]
fn parallel_strategy_helper_caches_java_strategy_objects() {
    let helper = ParallelStrategyHelper::load_instance();
    helper.clear_strategy_executor_map();

    let default_executor = helper.build_default_parallel_executor();
    let all_executor = helper.build_parallel_executor(Some(ParallelStrategyEnum::All));
    let any_executor = helper.build_parallel_executor(Some(ParallelStrategyEnum::Any));

    assert!(Arc::ptr_eq(&default_executor, &all_executor));
    assert!(!Arc::ptr_eq(&all_executor, &any_executor));
}

#[test]
fn script_engine_lifecycle_runs_after_real_build_and_can_be_cleaned() {
    let languages = Arc::new(Mutex::new(Vec::new()));
    let bus = FlowBus::new();
    bus.register_script_engine_init_hook(Arc::new(ScriptEngineInitHook {
        languages: Arc::clone(&languages),
    }));

    bus.register_script("lifecycle_script", "rhai", "40 + 2")
        .unwrap();
    assert_eq!(languages.lock().unwrap().as_slice(), ["rhai"]);

    // 编译失败时“初始化完成”并未发生，因此生命周期不能被误触发。
    assert!(
        bus.register_script("broken_script", "rhai", "let =")
            .is_err()
    );
    assert_eq!(languages.lock().unwrap().as_slice(), ["rhai"]);

    bus.clean_lifecycle_hooks();
    bus.register_script("after_clean", "rhai", "1 + 1").unwrap();
    assert_eq!(languages.lock().unwrap().as_slice(), ["rhai"]);
}

#[test]
fn chain_el_builder_validation_reports_precise_syntax_and_missing_node_errors() {
    let bus = FlowBus::new();
    bus.register("registered", cmp(|_| async { Ok(Value::Null) }));
    let builder = LiteFlowChainELBuilder::new(bus);

    assert!(builder.validate("THEN(registered)"));

    let missing = builder.validate_with_ex("THEN(registered,\n  absent)");
    assert!(!missing.is_success());
    let missing_error = missing.cause().unwrap().to_string();
    assert!(missing_error.contains("[absent] is not exist"));
    assert!(missing_error.contains("line 2, column 3"));
    assert!(missing_error.contains(" EL:   absent)"));
    assert!(
        missing_error
            .lines()
            .last()
            .is_some_and(|line| line.ends_with('^'))
    );

    let syntax = builder.validate_with_ex("THEN(registered, @)");
    assert!(!syntax.is_success());
    let syntax_error = syntax.cause().unwrap().to_string();
    assert!(syntax_error.contains("unexpected character: @"));
    assert!(syntax_error.contains("line 1, column 18"));
    assert!(syntax_error.contains(" EL: THEN(registered, @)"));
}

#[test]
fn loop_future_obj_preserves_executor_and_error_state() {
    let success = LoopFutureObj::success("loop-body");
    assert!(success.is_success());
    assert_eq!(success.executor_name(), "loop-body");
    assert!(success.ex().is_none());

    let mut failure = LoopFutureObj::fail("loop-body", LiteflowError::WhenExecute("boom".into()));
    assert!(!failure.is_success());
    assert_eq!(
        failure.ex().unwrap().to_string(),
        "when execute error: boom"
    );
    failure.set_executor_name("renamed-body");
    failure.set_success(true);
    failure.set_ex(None);
    assert_eq!(failure.executor_name(), "renamed-body");
    assert!(failure.is_success());
    assert!(failure.ex().is_none());
}

#[test]
fn data_bus_allocates_queries_context_and_releases_slot() {
    let slot = Arc::new(Slot::new(
        "data-bus-request".to_string(),
        "data-bus-chain",
        Value::Null,
    ));
    slot.beans.insert("answer".to_string(), Arc::new(42_u32));

    let slot_index = DataBus::offer_slot(slot.clone());
    assert!(Arc::ptr_eq(&DataBus::get_slot(slot_index).unwrap(), &slot));
    assert_eq!(DataBus::get_slot_index(&slot), Some(slot_index));
    let beans = DataBus::get_context_bean_list(slot_index);
    assert_eq!(beans.len(), 1);
    assert_eq!(beans[0].0, "answer");

    assert!(DataBus::release_slot(slot_index));
    assert!(DataBus::get_slot(slot_index).is_none());
    assert!(!DataBus::release_slot(slot_index));
}

#[tokio::test]
async fn flow_executor_registers_and_auto_releases_data_bus_slot() {
    let seen_slot_index = Arc::new(Mutex::new(None));
    let bus = FlowBus::new();
    bus.register(
        "data_bus_probe",
        DataBusAwareComponent {
            seen_slot_index: Arc::clone(&seen_slot_index),
        },
    );
    bus.add_chain("data_bus_chain", "THEN(data_bus_probe)")
        .unwrap();

    let response = bus.execute("data_bus_chain").await;
    assert!(response.is_success(), "{}", response.message);
    let slot_index = seen_slot_index
        .lock()
        .unwrap()
        .expect("component should observe slot index");
    assert!(DataBus::get_slot(slot_index).is_none());
}

#[tokio::test]
async fn cancelled_flow_future_still_releases_data_bus_slot() {
    let seen_slot_index = Arc::new(Mutex::new(None));
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let bus = FlowBus::new();
    bus.register(
        "cancellation_probe",
        CancellationProbeComponent {
            seen_slot_index: Arc::clone(&seen_slot_index),
            started: Mutex::new(Some(started_tx)),
        },
    );
    bus.add_chain("cancellation_chain", "THEN(cancellation_probe)")
        .unwrap();

    let execution = tokio::spawn(async move { bus.execute("cancellation_chain").await });
    tokio::time::timeout(std::time::Duration::from_secs(2), started_rx)
        .await
        .expect("component should start before timeout")
        .expect("component start signal should be delivered");
    let slot_index = seen_slot_index
        .lock()
        .unwrap()
        .expect("component should observe slot index");

    execution.abort();
    let _ = execution.await;
    assert!(DataBus::get_slot(slot_index).is_none());
}

#[tokio::test]
async fn parallel_loop_uses_loop_future_obj_for_success_and_failure() {
    let calls = Arc::new(AtomicUsize::new(0));
    let bus = FlowBus::new();
    let body_calls = Arc::clone(&calls);
    bus.register(
        "parallel_body",
        cmp(move |_| {
            let body_calls = Arc::clone(&body_calls);
            async move {
                body_calls.fetch_add(1, Ordering::SeqCst);
                Ok(Value::Null)
            }
        }),
    );
    bus.register(
        "parallel_failure",
        cmp(|_| async { Err(LiteflowError::WhenExecute("parallel loop failure".into())) }),
    );
    bus.add_chain(
        "parallel_loop_success",
        "FOR(3).parallel(true).DO(parallel_body)",
    )
    .unwrap();
    bus.add_chain(
        "parallel_loop_failure",
        "FOR(2).parallel(true).DO(parallel_failure)",
    )
    .unwrap();

    let success = bus.execute("parallel_loop_success").await;
    assert!(success.is_success(), "{}", success.message);
    assert_eq!(calls.load(Ordering::SeqCst), 3);

    let failure = bus.execute("parallel_loop_failure").await;
    assert!(!failure.is_success());
    assert!(failure.message.contains("parallel loop failure"));
}

#[test]
fn executable_objects_report_java_executeable_type() {
    let condition = ThenCondition::new();
    assert_eq!(condition.execute_type(), ExecuteableTypeEnum::Condition);

    let chain = Chain::new("chain", Vec::new());
    assert_eq!(Executable::execute_type(&chain), ExecuteableTypeEnum::Chain);

    let node = Node::new(
        NodeRef::new("node"),
        Arc::new(cmp(|_| async { Ok(Value::Null) })),
    );
    assert_eq!(node.execute_type(), ExecuteableTypeEnum::Node);
}

#[tokio::test]
async fn condition_hierarchy_dispatches_and_abstract_chain_refuses_execution() {
    let mut abstract_condition = AbstractCondition::new("template_chain");
    let condition: &dyn Condition = &abstract_condition;
    assert_eq!(condition.condition_type(), ConditionTypeEnum::Abstract);
    assert_eq!(condition.condition_id(), "condition-abstract");
    assert_eq!(condition.condition_tag(), None);

    abstract_condition.set_curr_chain_id("unresolved_chain");
    assert_eq!(abstract_condition.curr_chain_id(), "unresolved_chain");

    let ctx = liteflow_core::Ctx {
        inner: Arc::new(Slot::new(
            "abstract-request".to_string(),
            "unresolved_chain",
            Value::Null,
        )),
    };
    let error = abstract_condition
        .execute(&ctx, &liteflow_core::Frame::root())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        LiteflowError::ChainNotImplemented(message)
            if message == "chain[unresolved_chain] contains unimplemented variables, cannot be executed"
    ));
}

#[tokio::test]
async fn rollbackable_node_delegates_and_records_real_rollback_step() {
    let calls = Arc::new(AtomicUsize::new(0));
    let node = Node::new(
        NodeRef::new("rollback_node"),
        Arc::new(DirectRollbackComponent {
            calls: Arc::clone(&calls),
        }),
    );
    let ctx = liteflow_core::Ctx {
        inner: Arc::new(Slot::new(
            "rollback-request".to_string(),
            "rollback-chain",
            Value::Null,
        )),
    };

    node.rollback(&ctx, &liteflow_core::Frame::root())
        .await
        .unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let steps = ctx.inner.rollback_steps.lock().unwrap();
    assert_eq!(steps.len(), 1);
    assert!(steps[0].success);
    assert_eq!(steps[0].node_id, "rollback_node");
    assert_eq!(steps[0].node_name, "direct-rollback");
    assert!(steps[0].rollback_time_spent.is_some());
}

#[tokio::test]
async fn typed_node_component_objects_drive_real_control_flow() {
    let bus = FlowBus::new();
    bus.register(
        "boolean_node",
        NodeBooleanComponent::new("boolean", |_| async { Ok::<_, LiteflowError>(true) }),
    );
    bus.register(
        "for_node",
        NodeForComponent::new("for", |_| async { Ok::<_, LiteflowError>(2) }),
    );
    bus.register(
        "iterator_node",
        NodeIteratorComponent::new("iterator", |_| async {
            Ok::<_, LiteflowError>(vec![json!(2), json!(3)])
        }),
    );
    bus.register(
        "switch_node",
        NodeSwitchComponent::new("switch", |ctx: CmpContext| async move {
            let target_list = ctx.switch_target_list();
            ctx.set_data("switch_target_list", json!(target_list));
            Ok::<_, LiteflowError>("switch_target".to_string())
        }),
    );
    bus.register(
        "boolean_target",
        cmp(|ctx| async move {
            ctx.set_data("boolean_hit", json!(true));
            Ok(Value::Null)
        }),
    );
    bus.register(
        "loop_target",
        cmp(|ctx| async move {
            let count = ctx.get_data_as::<usize>("loop_count").unwrap_or_default();
            ctx.set_data("loop_count", json!(count + 1));
            Ok(Value::Null)
        }),
    );
    bus.register(
        "iterator_target",
        cmp(|ctx| async move {
            let sum = ctx.get_data_as::<i64>("iterator_sum").unwrap_or_default();
            let item = ctx.loop_object::<i64>().unwrap_or_default();
            ctx.set_data("iterator_sum", json!(sum + item));
            Ok(Value::Null)
        }),
    );
    bus.register(
        "switch_target",
        cmp(|ctx| async move {
            ctx.set_data("switch_hit", json!(true));
            Ok(Value::Null)
        }),
    );

    bus.add_chain("boolean_chain", "IF(boolean_node, boolean_target)")
        .unwrap();
    bus.add_chain("for_chain", "FOR(for_node).DO(loop_target)")
        .unwrap();
    bus.add_chain(
        "iterator_chain",
        "ITERATOR(iterator_node).DO(iterator_target)",
    )
    .unwrap();
    bus.add_chain("switch_chain", "SWITCH(switch_node).TO(switch_target)")
        .unwrap();

    assert_eq!(
        bus.execute("boolean_chain").await.data("boolean_hit"),
        Some(json!(true))
    );
    assert_eq!(
        bus.execute("for_chain").await.data("loop_count"),
        Some(json!(2))
    );
    assert_eq!(
        bus.execute("iterator_chain").await.data("iterator_sum"),
        Some(json!(5))
    );
    assert_eq!(
        bus.execute("switch_chain").await.data("switch_hit"),
        Some(json!(true))
    );
    assert_eq!(
        bus.execute("switch_chain").await.data("switch_target_list"),
        Some(json!(["switch_target"]))
    );
}

#[test]
fn instance_info_dto_keeps_java_camel_case_shape() {
    let spi = DefaultNodeInstanceIdManageSpiImpl::default();
    let info = spi.build_instance_info("chain1", "a", 1);
    let value = serde_json::to_value(&info).unwrap();

    assert_eq!(value["chainId"], "chain1");
    assert_eq!(value["nodeId"], "a");
    let instance_id = value["instanceId"].as_str().unwrap();
    let parts: Vec<&str> = instance_id.split('_').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], "a");
    assert_eq!(parts[1].len(), 8);
    assert!(parts[1].chars().all(|value| value.is_ascii_alphanumeric()));
    assert_eq!(parts[2], "1");
    assert_eq!(value["index"], 1);
}

#[test]
fn component_initializer_injects_java_metadata_and_global_retry_fallback() {
    let initializer = ComponentInitializer::with_default_retry_count(3);
    let component = initializer
        .init_component(
            Arc::new(cmp(|_| async { Ok(Value::Null) })),
            NodeTypeEnum::Boolean,
            Some("  inventory check  "),
            " inventory_node ",
        )
        .unwrap();

    assert_eq!(component.node_id(), "inventory_node");
    assert_eq!(component.node_type(), Some(NodeTypeEnum::Boolean));
    assert_eq!(component.name(), "inventory check");
    assert_eq!(component.retry_count(), 3);

    let error = match initializer.init_component(
        Arc::new(cmp(|_| async { Ok(Value::Null) })),
        NodeTypeEnum::Common,
        None,
        "   ",
    ) {
        Ok(_) => panic!("blank node id must be rejected"),
        Err(error) => error,
    };
    assert!(matches!(error, LiteflowError::NodeBuild(_)));
}

#[test]
fn instance_id_base_default_file_and_holder_execute_real_contract() {
    let directory = tempfile::tempdir().unwrap();
    let default_spi = Arc::new(DefaultNodeInstanceIdManageSpiImpl::with_base_path(
        directory.path(),
    ));
    let holder = NodeInstanceIdManageSpiHolder::new(default_spi.clone());

    let mut nodes = vec![
        Node::new(
            NodeRef::new("a"),
            Arc::new(cmp(|_| async { Ok(Value::Null) })),
        ),
        Node::new(
            NodeRef::new("a"),
            Arc::new(cmp(|_| async { Ok(Value::Null) })),
        ),
        Node::new(
            NodeRef::new("b"),
            Arc::new(cmp(|_| async { Ok(Value::Null) })),
        ),
    ];
    let current = holder.get_node_instance_id_manage_spi();
    let infos =
        BaseNodeInstanceIdManageSpi::assign_instance_ids(current.as_ref(), &mut nodes, "chain1");
    current
        .write_instance_id_file(&infos, "el-md5", "chain1")
        .unwrap();

    assert_eq!(
        current.read_instance_id_file("chain1").unwrap()[0],
        "el-md5"
    );
    let second_a = infos[1].instance_id().unwrap();
    assert_eq!(
        current.get_node_location_by_id("chain1", second_a).unwrap(),
        1
    );
    assert_eq!(
        current.get_node_instance_ids("chain1", "a").unwrap().len(),
        2
    );
    assert_eq!(
        BaseNodeInstanceIdManageSpi::get_node_by_id_and_instance_id(&nodes, second_a)
            .unwrap()
            .node_ref()
            .id,
        "a"
    );
    assert_eq!(
        BaseNodeInstanceIdManageSpi::get_node_by_id_and_index(&nodes, "a", 1)
            .unwrap()
            .node_instance_id(),
        Some(second_a)
    );

    let mut restored = vec![
        Node::new(
            NodeRef::new("a"),
            Arc::new(cmp(|_| async { Ok(Value::Null) })),
        ),
        Node::new(
            NodeRef::new("a"),
            Arc::new(cmp(|_| async { Ok(Value::Null) })),
        ),
    ];
    BaseNodeInstanceIdManageSpi::restore_instance_ids(&mut restored, "chain1", &infos);
    assert_eq!(restored[1].node_instance_id(), Some(second_a));

    holder.set_node_instance_id_manage_spi(Arc::new(PrefixInstanceIdSpi));
    assert_eq!(
        holder
            .get_node_instance_id_manage_spi()
            .gen_instance_id("chain2", "node", 3),
        "chain2:node:3"
    );
}

#[test]
fn validation_resp_retains_script_compile_error() {
    let executor = RhaiScriptExecutor::new();
    let success = executor.validate_with_ex("40 + 2");
    assert!(success.is_success());
    assert!(success.cause().is_none());

    let failure = executor.validate_with_ex("let value = ");
    assert!(!failure.is_success());
    assert!(failure.cause().is_some());
}

#[test]
fn script_executor_owns_real_load_unload_and_cache_lifecycle() {
    let executor = RhaiScriptExecutor::new();
    executor.load("b", "1 + 1").unwrap();
    executor.load("a", "40 + 2").unwrap();
    assert_eq!(
        executor.node_ids().unwrap(),
        vec!["a".to_string(), "b".to_string()]
    );

    let ctx = liteflow_core::CmpContext {
        inner: Arc::new(Slot::new(
            "script-request".to_string(),
            "script-chain",
            Value::Null,
        )),
        node: NodeRef::new("a"),
        frame: liteflow_core::Frame::root(),
    };
    assert_eq!(executor.execute("a", &ctx).unwrap(), json!(42));

    executor.unload("a").unwrap();
    let error = executor.execute("a", &ctx).unwrap_err();
    assert!(matches!(
        error,
        LiteflowError::Script { node, msg }
            if node == "a" && msg == "script for node[a] is not loaded"
    ));

    executor.clean_cache().unwrap();
    assert!(executor.node_ids().unwrap().is_empty());
}

#[test]
fn flow_init_hook_executes_every_registered_supplier() {
    FlowInitHook::clean_hook();
    let calls = Arc::new(AtomicUsize::new(0));
    for _ in 0..2 {
        let calls = Arc::clone(&calls);
        FlowInitHook::add_hook(move || {
            calls.fetch_add(1, Ordering::SeqCst);
            true
        });
    }

    assert_eq!(FlowInitHook::len(), 2);
    FlowInitHook::execute_hook();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    FlowInitHook::clean_hook();
    assert!(FlowInitHook::is_empty());
}

#[tokio::test]
async fn flow_executor_holder_loads_and_executes_real_chain() {
    FlowExecutorHolder::clean();
    assert!(FlowExecutorHolder::load_instance().is_err());

    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    bus.add_chain("holder_chain", "THEN(a)").unwrap();
    let _ = FlowExecutorHolder::load_instance_with_bus(bus);

    let response = FlowExecutorHolder::load_instance()
        .unwrap()
        .execute("holder_chain")
        .await;
    assert!(response.is_success(), "{}", response.message);
    FlowExecutorHolder::clean();
}

#[test]
fn flow_executor_configuration_updates_the_core_global_getter() {
    LiteflowConfigGetter::clean();
    let bus = FlowBus::new();
    let mut configured = LiteflowConfig::default();
    configured.set_slot_size(257);

    let executor = FlowExecutor::new_with_config(bus, configured.clone());
    assert_eq!(executor.liteflow_config(), configured);
    assert_eq!(LiteflowConfigGetter::get(), configured);

    let mut updated = configured;
    updated.set_slot_size(513);
    executor.set_liteflow_config(updated.clone());
    assert_eq!(executor.liteflow_config(), updated);
    assert_eq!(LiteflowConfigGetter::get(), updated);
    LiteflowConfigGetter::clean();
}

#[tokio::test]
async fn flow_bus_java_registry_lifecycle_uses_real_state_and_script_unload() {
    let bus = FlowBus::new();
    let cloned_bus = bus.clone();

    // 所有克隆共享 Java initStat 对等状态。
    assert!(bus.need_init());
    assert!(!cloned_bus.need_init());
    cloned_bus.clear_stat();
    assert!(bus.need_init());

    let initialized = ComponentInitializer::load_instance()
        .init_component(
            Arc::new(cmp(|_| async { Ok(Value::Null) })),
            NodeTypeEnum::Common,
            Some("托管节点"),
            "managed",
        )
        .unwrap();
    bus.add_managed_node("managed", initialized).unwrap();
    assert!(bus.contain_node("managed"));
    assert_eq!(bus.get_node_map().len(), 1);

    let phase_one = Chain::new("phase_one", Vec::new());
    bus.add_chain_phase1(phase_one);
    assert!(bus.contain_chain("phase_one"));
    assert_eq!(bus.get_chain_map().len(), 1);

    bus.add_script_node(
        "script_node",
        Some("真实 Rhai 脚本"),
        NodeTypeEnum::Script,
        "40 + 2",
        "rhai",
    )
    .unwrap();
    bus.add_chain("script_chain", "THEN(script_node)").unwrap();
    assert!(bus.execute("script_chain").await.is_success());
    assert!(bus.unload_script_node("script_node").unwrap());
    assert!(!bus.contain_node("script_node"));
    assert!(!bus.unload_script_node("script_node").unwrap());

    bus.clean_cache().unwrap();
    assert!(bus.get_node_map().is_empty());
    assert!(bus.get_chain_map().is_empty());
}

#[test]
fn flow_bus_refreshes_el_metadata_and_exposes_fallback_snapshot() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    let ids = bus
        .refresh_flow_meta_data(
            FlowParserTypeEnum::TypeElJson,
            r#"{"flow":{"chain":[{"id":"refreshed","body":"THEN(a)"}]}}"#,
        )
        .unwrap();
    assert_eq!(ids, vec!["refreshed"]);
    assert!(bus.contain_chain("refreshed"));

    bus.register_fallback(
        "fallback",
        NodeTypeEnum::Common,
        cmp(|_| async { Ok(Value::Null) }),
    )
    .unwrap();
    assert!(bus.get_fall_back_node(NodeTypeEnum::Common).is_some());
    assert!(bus.remove_node("fallback"));
    assert!(!bus.remove_node("fallback"));
}

#[tokio::test]
async fn default_route_namespace_matches_java_lowercase_constant() {
    assert_eq!(ChainConstant::DEFAULT_NAMESPACE, "default");
    assert_eq!(LocalDefaultFlowConstant::DEFAULT, "default");

    let bus = FlowBus::new();
    bus.register("route", cmp(|_| async { Ok(Value::Bool(true)) }));
    bus.register("body", cmp(|_| async { Ok(Value::Null) }));
    bus.add_route_chain(
        "default_route",
        ChainConstant::DEFAULT_NAMESPACE,
        "route",
        "body",
    )
    .unwrap();

    let responses = bus
        .execute_route_chain(None, json!({"id": 1}))
        .await
        .unwrap();
    assert_eq!(responses.len(), 1);
    assert!(responses[0].is_success());
}
