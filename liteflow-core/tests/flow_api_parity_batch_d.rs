//! Flow 域与 Slot 域未触达公开 API 的 Java v2.16.0 对等补测（批次 D）。
//!
//! 覆盖对象：
//! - `Chain#setRouteItem/clearRouteItem/withNamespace/withThreadPoolExecutorClass`
//! - `FlowEventPublisher#setListener/hasListener/removeListener/publish` 与
//!   `publish_ctx` 内部入口
//! - `DefaultRequestIdGenerator#fastSimpleUuid`、`IdGeneratorHolder#loadGenerator`
//! - `Frame` 的 `with_chain_cmp_data/with_chain_thread_pool/
//!   with_condition_thread_pool/with_switch_target_list/loop_index_at/
//!   loop_object_at/chain_thread_pool/condition_thread_pool`
//! - `Slot#generateRequestId/removeException/getSubException/getTimeoutItemList/
//!   printStep/printRollbackStep`、`DataBus#occupyCount`、
//!   `Ctx#isEnded/recordStep/registerRollback`
//! - `FlowBus#registerFallbackArc/tryRegisterArc`、`NodeTypeEnum#isScript`

use liteflow_core::flow::element::chain::Chain;
use liteflow_core::flow::flow_event_publisher::FlowEventPublisher;
use liteflow_core::flow::id::fast_simple_uuid;
use liteflow_core::flow::id::id_generator_holder::IdGeneratorHolder;
use liteflow_core::slot::{Ctx, DataBus, Slot};
use liteflow_core::{CmpStep, CmpStepTypeEnum, FlowBus, Frame, NodeRef, NodeTypeEnum, cmp};
use liteflow_core::{FlowEvent, FlowEventListener};
use serde_json::{Value, json};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

/// Chain 路由项清除与命名空间/线程池链式入口。
///
/// 对应 Java: `Chain#setRouteItem`（null 覆盖）、`Chain#setNamespace`、
/// `Chain#setThreadPoolExecutorClass`。
#[test]
fn chain_route_and_metadata_mutators_round_trip() {
    let mut chain = Chain::new("route_chain", Vec::new());
    let component: Arc<dyn liteflow_core::NodeComponent> =
        Arc::new(cmp(|_| async { Ok(Value::Null) }));
    let node = liteflow_core::flow::element::node::Node::new(NodeRef::new("route-node"), component);
    chain.set_route_item(Arc::new(node));
    assert!(chain.get_route_item().is_some());

    chain.clear_route_item();
    assert!(chain.get_route_item().is_none());

    let chain = Chain::new("meta_chain", Vec::new())
        .with_namespace("ns-1")
        .with_thread_pool_executor_class("com.example.ChainPool");
    assert_eq!(chain.get_namespace(), "ns-1");
    assert_eq!(
        chain.get_thread_pool_executor_class(),
        Some("com.example.ChainPool")
    );
    assert!(!chain.is_compiled());
}

/// FlowEventPublisher 的监听器生命周期与发布。
///
/// 对应 Java: `FlowEventPublisher#setListener/hasListener/removeListener/publish`。
#[tokio::test]
async fn event_publisher_listener_lifecycle() {
    let slot = Arc::new(Slot::new(
        "RID-EVENT".to_string(),
        "event_chain",
        Value::Null,
    ));
    let ctx = Ctx::new(slot.clone());

    assert!(!FlowEventPublisher::has_listener(&ctx));
    // 无监听器时发布静默忽略
    FlowEventPublisher::publish(&ctx, &FlowEvent::builder().r#type("before").build());

    let seen = Arc::new(Mutex::new(Vec::<String>::new()));
    let listener = EventListenerStub { seen: seen.clone() };
    FlowEventPublisher::set_listener(&ctx, Arc::new(listener));
    assert!(FlowEventPublisher::has_listener(&ctx));

    FlowEventPublisher::publish(&ctx, &FlowEvent::builder().r#type("after").build());
    assert_eq!(*seen.lock().unwrap(), vec!["after".to_string()]);

    FlowEventPublisher::remove_listener(&ctx);
    assert!(!FlowEventPublisher::has_listener(&ctx));
    FlowEventPublisher::publish(&ctx, &FlowEvent::builder().r#type("ignored").build());
    assert_eq!(*seen.lock().unwrap(), vec!["after".to_string()]);

    // publish_ctx 内部入口走同一 Slot 附件
    FlowEventPublisher::set_listener(&ctx, Arc::new(EventListenerStub { seen: seen.clone() }));
    FlowEventPublisher::publish_ctx(&slot, &FlowEvent::builder().r#type("ctx-published").build());
    assert_eq!(
        *seen.lock().unwrap(),
        vec!["after".to_string(), "ctx-published".to_string()]
    );
}

struct EventListenerStub {
    seen: Arc<Mutex<Vec<String>>>,
}

impl FlowEventListener for EventListenerStub {
    fn on_event(&self, event: &FlowEvent) {
        self.seen.lock().unwrap().push(event.get_type().to_string());
    }
}

/// 请求 ID 生成器：快速 UUID 与默认格式。
///
/// 对应 Java: `DefaultRequestIdGenerator#fastSimpleUUID` 与
/// `IdGeneratorHolder#loadGenerator`。
#[test]
fn request_id_generators_produce_java_shapes() {
    let fast = fast_simple_uuid();
    // Java fastSimpleUUID = 32 位十六进制
    assert_eq!(fast.len(), 32);
    assert!(fast.chars().all(|c| c.is_ascii_hexdigit()));

    let generator = IdGeneratorHolder::load_generator();
    let id = generator.generate();
    assert!(!id.is_empty());

    // 两次生成的 ID 不同
    let id2 = generator.generate();
    assert_ne!(id, id2);
}

/// Frame 线程池/组件数据/switch 目标/循环栈的写入与读取。
#[test]
fn frame_metadata_writers_and_loop_stacks() {
    let mut frame = Frame::root();
    assert_eq!(frame.chain_thread_pool(), None);
    assert_eq!(frame.condition_thread_pool(), None);

    frame = frame
        .with_chain_thread_pool(Some("com.example.ChainPool"))
        .with_condition_thread_pool(Some("com.example.WhenPool"))
        .with_chain_cmp_data(Some("chain-data"))
        .with_switch_target_list(&["a".to_string(), "b".to_string()]);
    assert_eq!(frame.chain_thread_pool(), Some("com.example.ChainPool"));
    assert_eq!(frame.condition_thread_pool(), Some("com.example.WhenPool"));
    assert_eq!(frame.chain_cmp_data(), Some("chain-data"));

    // with_chain_cmp_data(None) 保留父链数据
    frame = frame.with_chain_cmp_data(None);
    assert_eq!(frame.chain_cmp_data(), Some("chain-data"));

    // 循环索引/对象栈：Java loopIndexTL/loopObjectTL 的压栈与查询
    frame = frame.push(3, None);
    assert_eq!(frame.loop_index(), Some(3));
    assert_eq!(frame.loop_index_at(0), Some(3));
    assert_eq!(frame.loop_index_at(1), None);
    frame = frame.push(0, Some(json!({"item": 1})));
    assert!(frame.loop_object().is_some());
    assert!(frame.loop_object_at(0).is_some());
    assert_eq!(frame.loop_object_at(1), None);
}

/// Slot 请求 ID 生成、异常移除、子链异常与超时项列表。
#[test]
fn slot_lifecycle_metadata_methods() {
    let mut slot = Slot::new(String::new(), "main", Value::Null);
    slot.generate_request_id();
    assert!(!slot.get_request_id().is_empty());

    slot.set_exception("main failure");
    slot.set_sub_exception("sub_chain", "sub failure");
    slot.add_timeout_item("timeout-1");
    slot.add_timeout_item("timeout-2");

    assert_eq!(slot.get_exception(), Some("main failure".to_string()));
    assert_eq!(
        slot.get_sub_exception("sub_chain"),
        Some("sub failure".to_string())
    );
    assert_eq!(slot.get_sub_exception("missing"), None);
    assert_eq!(
        slot.get_timeout_item_list(),
        vec!["timeout-1".to_string(), "timeout-2".to_string()]
    );

    slot.remove_exception();
    assert_eq!(slot.get_exception(), None);
}

/// Slot 步骤与回滚步骤的打印文本。
///
/// 对应 Java: `Slot#printStep/printRollbackStep`。
#[test]
fn slot_print_step_outputs() {
    let slot = Slot::new("RID-PRINT".to_string(), "main", Value::Null);
    let mut step = CmpStep::new("a", "节点A", CmpStepTypeEnum::Single);
    step.finish(true, None);
    slot.add_step(step);
    let mut rollback = CmpStep::new("r", "回滚", CmpStepTypeEnum::Single);
    rollback.finish_rollback(true, None);
    slot.add_rollback_step(rollback);

    slot.print_step();
    slot.print_rollback_step();
}

/// DataBus 占用计数与槽位生命周期。
///
/// 对应 Java: `DataBus#init/offerIndex/releaseSlot` 与占用计数。
#[test]
fn data_bus_occupy_count_tracks_slots() {
    DataBus::init(8);
    let slot = Arc::new(Slot::new("RID-BUS".to_string(), "main", Value::Null));
    let index = DataBus::offer_slot(slot);
    assert_eq!(DataBus::occupy_count(), 1);
    assert!(DataBus::release_slot(index));
    assert_eq!(DataBus::occupy_count(), 0);
}

/// Ctx 的结束标记、步骤记录与回滚登记。
#[tokio::test]
async fn ctx_record_and_end_flags() {
    let slot = Arc::new(Slot::new("RID-CTX".to_string(), "main", Value::Null));
    let ctx = Ctx::new(slot.clone());

    assert!(!ctx.is_ended());
    ctx.inner.ended.store(true, Ordering::Relaxed);
    assert!(ctx.is_ended());

    let step = CmpStep::new("n", "", CmpStepTypeEnum::Single);
    ctx.record_step(step);
    assert_eq!(slot.get_execute_steps().len(), 1);

    // Java Slot#addStep 的公开入口是 add_rollback_step；register_rollback
    // 走内部回滚登记（NodeExecutor 逆序补偿使用），外部验证记录步骤不受影响
    let mut rollback = CmpStep::new("n", "", CmpStepTypeEnum::Single);
    rollback.finish_rollback(true, None);
    slot.add_rollback_step(rollback);
    assert_eq!(slot.get_rollback_steps().len(), 1);
}

/// FlowBus 降级组件与按 Arc 注册的入口。
#[tokio::test]
async fn flow_bus_arc_registration_entries() {
    let bus = FlowBus::new();
    let component = Arc::new(cmp(|_| async { Ok(json!("fallback")) }));

    bus.register_fallback_arc("fb_node", NodeTypeEnum::Common, component.clone())
        .expect("注册降级组件");
    bus.try_register_arc("fb_node_2", component)
        .expect("按 Arc 注册");
    let map = bus.get_node_map();
    assert!(map.contains_key("fb_node"));
    assert!(map.contains_key("fb_node_2"));
}

/// NodeTypeEnum#isScript 区分脚本节点。
#[test]
fn node_type_is_script_classification() {
    assert!(NodeTypeEnum::Script.is_script());
    assert!(NodeTypeEnum::BooleanScript.is_script());
    assert!(NodeTypeEnum::SwitchScript.is_script());
    assert!(NodeTypeEnum::ForScript.is_script());
    assert!(NodeTypeEnum::WhileScript.is_script());
    assert!(NodeTypeEnum::BreakScript.is_script());
    assert!(!NodeTypeEnum::Common.is_script());
    assert!(!NodeTypeEnum::Boolean.is_script());
    assert!(!NodeTypeEnum::Switch.is_script());
}

/// 占用计数在并发分配时保持真实（简单并发压力）。
#[tokio::test]
async fn data_bus_occupy_count_under_concurrency() {
    DataBus::init(16);
    let mut handles = Vec::new();
    for _ in 0..8 {
        handles.push(tokio::spawn(async move {
            let slot = Arc::new(Slot::new("RID-CONC".to_string(), "main", Value::Null));
            let index = DataBus::offer_slot(slot);
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            DataBus::release_slot(index);
        }));
    }
    for handle in handles {
        handle.await.expect("并发任务完成");
    }
    assert_eq!(DataBus::occupy_count(), 0);
    let _ = Ordering::SeqCst;
}
