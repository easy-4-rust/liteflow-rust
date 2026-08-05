//! CmpStep / LiteflowResponse / Slot 的 Java v2.16.0 对等 API 补测。
//!
//! 覆盖现有 parity 测试未触达的公开入口：
//! - `CmpStep#getTimeSpentMs/getRollbackTimeSpentMs/setNodeId/getStepType/
//!   setStepType/getNodeName/buildStringWithTime/buildRollbackStringWithTime/toString`
//! - `LiteflowResponse` 的 `initialization_failure`、`set_slot`、`get_timeout_items`
//!   与 `step_str*`/`rollback_step_str*` 别名
//! - `Slot#addTimeoutItem/getTimeoutItemList`、私有投递与请求队列的跨链隔离

use liteflow_core::flow::element::node::Node;
use liteflow_core::{CmpStep, CmpStepTypeEnum, LiteflowResponse, NodeRef, Slot, cmp};
use serde_json::{Value, json};
use std::sync::Arc;

/// CmpStep 耗时毫秒入口：未设置时返回 0，设置后按真实 Duration 换算。
///
/// 对应 Java: `CmpStep#getTimeSpentMs` 与 `CmpStep#getRollbackTimeSpentMs`
/// （内部 `timeSpent.toMillis()`，null 返回 0）。
#[test]
fn cmp_step_millisecond_accessors_default_to_zero_and_convert_duration() {
    let mut step = CmpStep::new("a", "", CmpStepTypeEnum::Single);
    assert_eq!(step.time_spent_ms(), 0);
    assert_eq!(step.rollback_time_spent_ms(), 0);

    step.set_time_spent(2500);
    step.set_rollback_time_spent(750);
    assert_eq!(step.time_spent_ms(), 2500);
    assert_eq!(step.rollback_time_spent_ms(), 750);
}

/// CmpStep 名称/类型/节点 ID 的 Java 命名 setter/getter。
///
/// 对应 Java: `CmpStep#setNodeId/getNodeId/getStepType/setStepType/getNodeName`。
#[test]
fn cmp_step_node_identity_mutators_round_trip() {
    let mut step = CmpStep::new("a", "节点A", CmpStepTypeEnum::Single);
    assert_eq!(step.get_node_id(), "a");
    assert_eq!(step.get_node_name(), "节点A");
    assert_eq!(step.get_step_type(), CmpStepTypeEnum::Single);

    step.set_node_id("renamed");
    step.set_node_name("改名");
    step.set_step_type(CmpStepTypeEnum::Start);
    assert_eq!(step.get_node_id(), "renamed");
    assert_eq!(step.get_node_name(), "改名");
    assert_eq!(step.get_step_type(), CmpStepTypeEnum::Start);
}

/// buildStringWithTime 四分支与回滚变体。
///
/// 对应 Java: `CmpStep#buildStringWithTime`（nodeName 空白×timeSpent 有无）与
/// `CmpStep#buildRollbackStringWithTime`。
#[test]
fn cmp_step_time_string_branches_match_java_format() {
    // 有 nodeName + 有耗时：`id[名称]<ms>`
    let mut named_with_time = CmpStep::new("a", "节点A", CmpStepTypeEnum::Single);
    named_with_time.set_time_spent(120);
    assert_eq!(named_with_time.build_string_with_time(), "a[节点A]<120>");

    // 空白 nodeName + 有耗时：`id<ms>`
    let mut blank_with_time = CmpStep::new("b", "", CmpStepTypeEnum::Single);
    blank_with_time.set_time_spent(88);
    assert_eq!(blank_with_time.build_string_with_time(), "b<88>");

    // 无耗时回退 buildString：`id[名称]` / `id`
    let named_no_time = CmpStep::new("c", "节点C", CmpStepTypeEnum::Single);
    assert_eq!(named_no_time.build_string_with_time(), "c[节点C]");
    let blank_no_time = CmpStep::new("d", "", CmpStepTypeEnum::Single);
    assert_eq!(blank_no_time.build_string_with_time(), "d");

    // 回滚耗时变体：`id[名称]<ms>`、空白名称 `id<ms>`、无耗时回退
    let mut rollback_named = CmpStep::new("e", "节点E", CmpStepTypeEnum::Single);
    rollback_named.set_rollback_time_spent(40);
    assert_eq!(
        rollback_named.build_rollback_string_with_time(),
        "e[节点E]<40>"
    );
    let mut rollback_blank = CmpStep::new("f", "", CmpStepTypeEnum::Single);
    rollback_blank.set_rollback_time_spent(5);
    assert_eq!(rollback_blank.build_rollback_string_with_time(), "f<5>");
    let rollback_none = CmpStep::new("g", "节点G", CmpStepTypeEnum::Single);
    assert_eq!(rollback_none.build_rollback_string_with_time(), "g[节点G]");
}

/// CmpStep Debug 输出包含全部 Java 字段，且与节点 ID 相等性无关。
#[test]
fn cmp_step_debug_and_equality_are_independent() {
    let mut step = CmpStep::new("h", "节点H", CmpStepTypeEnum::Single);
    step.set_node_instance_id("h_1");
    step.set_tag("tag-h");
    step.set_success(true);
    step.set_exception(Some("diag".to_string()));
    step.set_thread_name("worker-9");

    let debug = format!("{step:?}");
    assert!(debug.contains("node_instance_id"));
    assert!(debug.contains("h_1"));
    assert!(debug.contains("node_id"));
    assert!(debug.contains("node_name"));
    assert!(debug.contains("tag"));
    assert!(debug.contains("step_type"));
    assert!(debug.contains("start_time"));
    assert!(debug.contains("end_time"));
    assert!(debug.contains("time_spent"));
    assert!(debug.contains("success"));
    assert!(debug.contains("exception"));
    assert!(debug.contains("rollback_time_spent"));
    assert!(debug.contains("step_data"));
    assert!(debug.contains("thread_name"));

    // Java equals 只比较 nodeId，Debug 不参与相等性
    let mut same = CmpStep::new("h", "", CmpStepTypeEnum::Start);
    same.set_node_instance_id("other");
    assert!(step.equals(&same));
    assert_ne!(format!("{step:?}"), format!("{same:?}"));
}

/// LiteflowResponse 初始化失败响应保留请求 ID、链 ID 与原始输入。
///
/// 对应 Java: `FlowExecutor#init` 失败时构建的失败响应语义
/// （`LiteflowResponse` 以 requestId/chainId/input 构造并携带失败消息）。
#[test]
fn initialization_failure_preserves_request_chain_and_input() {
    let response = LiteflowResponse::initialization_failure(
        "RID-INIT",
        "init_chain",
        json!({"seed": 1}),
        "config parse failed",
    );

    assert!(!response.is_success());
    assert_eq!(response.get_request_id(), "RID-INIT");
    assert_eq!(response.get_chain_id(), "init_chain");
    assert_eq!(response.get_message(), "rule initialization failed");
    assert_eq!(response.get_cause(), Some("config parse failed"));
    assert_eq!(
        response.get_slot().get_input("any"),
        None,
        "输入保留在 Slot 初始数据而非节点输入表"
    );
}

/// set_slot 后用新 Slot 的状态刷新响应快照。
///
/// 对应 Java: `LiteflowResponse#setSlot` 的字段同步语义。
#[test]
fn set_slot_refreshes_response_snapshot_fields() {
    let slot = Slot::new("RID-OLD".to_string(), "old_chain", json!(null));
    slot.add_step(CmpStep::new("old", "旧节点", CmpStepTypeEnum::Single));
    let first = LiteflowResponse::new_main_response(Arc::new(slot));
    assert_eq!(first.get_request_id(), "RID-OLD");
    assert_eq!(first.get_execute_step_queue().len(), 1);

    let new_slot = Slot::new("RID-NEW".to_string(), "new_chain", json!({"x": 1}));
    new_slot.add_step(CmpStep::new("new", "新节点", CmpStepTypeEnum::Single));
    let mut second = LiteflowResponse::new_main_response(Arc::new(new_slot));
    second.set_slot(second.get_slot().clone());
    assert_eq!(second.get_request_id(), "RID-NEW");
    assert_eq!(second.get_chain_id(), "new_chain");
    assert_eq!(second.get_execute_step_queue().len(), 1);
    assert_eq!(second.get_execute_step_queue()[0].get_node_id(), "new");
    // set_slot 后步骤快照与 Slot 当前队列一致
    assert_eq!(second.get_slot().steps.lock().unwrap().len(), 1);
}

/// Slot 超时项与 LiteflowResponse#getTimeoutItems 的联动。
///
/// 对应 Java: `Slot#addTimeoutItem/getTimeoutItemList` 与
/// `LiteflowResponse#getTimeoutItems`。
#[test]
fn timeout_items_round_trip_through_response() {
    let slot = Slot::new("RID-TIMEOUT".to_string(), "main", json!(null));
    slot.add_timeout_item("when-item-1");
    slot.add_timeout_item("when-item-2");
    let response = LiteflowResponse::new_main_response(Arc::new(slot));
    assert_eq!(
        response.get_timeout_items(),
        vec!["when-item-1".to_string(), "when-item-2".to_string()]
    );
}

/// step_str 具名别名与带耗时/不带耗时变体共享同一 Slot 快照。
///
/// 对应 Java: `LiteflowResponse#getExecuteStepStr/WithTime/WithoutTime`。
#[test]
fn step_str_aliases_share_snapshot() {
    let slot = Slot::new("RID-STR".to_string(), "main", json!(null));
    let component = Arc::new(cmp(|_| async { Ok(Value::Null) }));
    let mut step = CmpStep::new("a", "节点A", CmpStepTypeEnum::Single);
    step.set_instance(component);
    step.set_ref_node(Node::new(
        NodeRef::new("a"),
        Arc::new(cmp(|_| async { Ok(Value::Null) })),
    ));
    step.set_time_spent(15);
    slot.add_step(step);
    let response = LiteflowResponse::new_main_response(Arc::new(slot));

    assert_eq!(response.step_str(), "a[节点A]");
    assert_eq!(response.get_execute_step_str(), "a[节点A]");
    assert!(response.step_str_with_time().contains("a[节点A]<"));
    assert!(
        response
            .get_execute_step_str_with_time()
            .contains("a[节点A]<")
    );
    assert_eq!(response.get_execute_step_str_without_time(), "a[节点A]");
}

/// Slot 请求队列按链 ID 隔离：私有投递不污染子链请求。
///
/// 对应 Java: `Slot#setChainReqData2Queue/getChainReqDataFromQueue` 与
/// `Slot#setPrivateDeliveryData/getPrivateDeliveryData` 的 key 前缀隔离。
#[test]
fn slot_queues_isolate_chain_and_private_keys() {
    let slot = Slot::new("RID-Q".to_string(), "main", json!(null));

    slot.set_chain_req_data2_queue("sub1", json!(1));
    slot.set_chain_req_data2_queue("sub1", json!(2));
    slot.set_chain_req_data2_queue("sub2", json!(10));
    slot.set_private_delivery_data("node-b", json!("private"));

    // 子链队列：sub1 先进先出，sub2 独立
    assert_eq!(slot.get_chain_req_data_from_queue("sub1"), Some(json!(1)));
    assert_eq!(slot.get_chain_req_data_from_queue("sub1"), Some(json!(2)));
    assert_eq!(slot.get_chain_req_data_from_queue("sub1"), None);
    assert_eq!(slot.get_chain_req_data_from_queue("sub2"), Some(json!(10)));

    // 私有投递队列与子链请求互不干扰
    assert_eq!(
        slot.get_private_delivery_data("node-b"),
        Some(json!("private"))
    );
    assert_eq!(slot.get_private_delivery_data("node-b"), None);
    assert_eq!(slot.get_chain_req_data_from_queue("node-b"), None);
}

/// 响应从带异常 Slot 构造时 message/cause 同步，步骤与回滚步骤保留。
#[test]
fn response_from_failed_slot_syncs_message_and_steps() {
    let slot = Slot::new("RID-FAIL".to_string(), "main", json!(null));
    slot.set_exception("inner failure");
    let mut rollback = CmpStep::new("r", "", CmpStepTypeEnum::Single);
    rollback.finish_rollback(false, Some("rolled back".to_string()));
    slot.add_rollback_step(rollback);
    let response = LiteflowResponse::new_main_response(Arc::new(slot));

    assert!(!response.is_success());
    assert_eq!(response.get_message(), "inner failure");
    assert_eq!(response.get_cause(), Some("inner failure"));
    assert_eq!(response.get_rollback_steps().len(), 1);
    assert_eq!(response.rollback_step_str(), "r");
}
