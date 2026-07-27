//! `LiteflowResponse` Java v2.16 对等语义回归测试。
//!
//! 对应 Java:
//! `com.yomahub.liteflow.flow.LiteflowResponse`。

use liteflow_core::flow::element::node::Node;
use liteflow_core::{
    CmpStep, CmpStepTypeEnum, ExecuteOption, FlowBus, LiteflowResponse, NodeRef, Slot, cmp,
};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

#[tokio::test]
async fn real_execution_preserves_slot_steps_context_and_java_accessors() {
    let bus = FlowBus::new();
    bus.register(
        "a",
        cmp(|ctx| async move {
            ctx.set_step_data(json!({"source": "component"}));
            Ok(Value::Null)
        }),
    );
    bus.add_chain("response_chain", "THEN(a, a)").unwrap();

    let response = bus
        .execute_with_option(
            "response_chain",
            json!({"request": true}),
            ExecuteOption::of()
                .request_id("RID-RESPONSE")
                .conversation_id("CID-RESPONSE")
                .context_bean("first", Arc::new(7_u32))
                .context_bean("second", Arc::new(String::from("value"))),
        )
        .await;

    assert!(response.is_success());
    assert_eq!(response.get_request_id(), "RID-RESPONSE");
    assert_eq!(response.get_conversation_id(), Some("CID-RESPONSE"));
    assert_eq!(response.get_chain_id(), "response_chain");
    assert_eq!(*response.get_first_context_bean::<u32>().unwrap(), 7);
    assert_eq!(
        response
            .get_context_bean::<String>("second")
            .unwrap()
            .as_str(),
        "value"
    );
    assert_eq!(*response.get_context_bean_by_type::<u32>().unwrap(), 7);
    assert_eq!(response.get_execute_step_queue().len(), 2);
    assert_eq!(response.get_execute_steps().len(), 1);
    assert_eq!(response.get_execute_steps()[0].0, "a");
    assert_eq!(response.get_execute_steps()[0].1.len(), 2);
    assert_eq!(response.get_execute_step_str(), "a==>a");
    assert!(response.get_execute_step_str_with_time().contains("a<"));
    let first_step = &response.get_execute_step_queue()[0];
    assert_eq!(
        first_step.get_step_data(),
        Some(&json!({"source": "component"}))
    );
    assert!(first_step.get_instance().is_some());
    assert_eq!(first_step.get_ref_node().unwrap().get_id(), "a");
    assert!(first_step.get_end_time().is_some());
    assert!(!first_step.get_thread_name().is_empty());

    // 响应创建不能再从 Slot 中 take 掉步骤；子链合并和诊断仍需读取同一队列。
    assert_eq!(response.get_slot().steps.lock().unwrap().len(), 2);
}

#[test]
fn cmp_step_java_mutation_equality_and_runtime_references_are_aligned() {
    let component = Arc::new(cmp(|_| async { Ok(Value::Null) }));
    let node = Node::new(NodeRef::new("node"), component.clone()).with_instance_id("node_1");
    let start_time = UNIX_EPOCH + Duration::from_secs(10);
    let end_time = UNIX_EPOCH + Duration::from_secs(11);

    let mut step = CmpStep::new("node", "节点", CmpStepTypeEnum::Single);
    step.set_instance(component);
    step.set_ref_node(node);
    step.set_tag("tag");
    step.set_start_time(start_time);
    step.set_end_time(end_time);
    step.set_time_spent(17);
    step.set_rollback_time_spent(9);
    step.set_step_data(json!({"attempt": 2}));
    step.set_thread_name("worker-1");
    step.set_success(true);
    step.set_exception(Some("diagnostic".to_string()));

    assert_eq!(step.get_node_instance_id(), Some("node_1"));
    assert!(step.get_instance().is_some());
    assert_eq!(step.get_ref_node().unwrap().get_id(), "node");
    assert_eq!(step.get_tag(), Some("tag"));
    assert_eq!(step.get_start_time(), start_time);
    assert_eq!(step.get_end_time(), Some(end_time));
    assert_eq!(step.get_time_spent(), Some(17));
    assert_eq!(step.get_rollback_time_spent(), Some(9));
    assert_eq!(step.get_step_data(), Some(&json!({"attempt": 2})));
    assert_eq!(step.get_thread_name(), "worker-1");
    assert!(step.is_success());
    assert_eq!(step.get_exception(), Some("diagnostic"));

    // Java equals 只比较 nodeId；其他运行信息不参与相等性。
    let same_node = CmpStep::new("node", "另一个名称", CmpStepTypeEnum::Start);
    let different_node = CmpStep::new("other", "节点", CmpStepTypeEnum::Single);
    assert!(step.equals(&same_node));
    assert_eq!(step, same_node);
    assert_ne!(step, different_node);

    let mut cloned = step.clone();
    cloned.set_step_data(json!({"attempt": 3}));
    cloned.set_thread_name("worker-2");
    assert_eq!(step.get_step_data(), Some(&json!({"attempt": 2})));
    assert_eq!(step.get_thread_name(), "worker-1");
}

#[test]
fn factories_group_rollback_steps_and_preserve_inner_exception() {
    let mut slot = Slot::new("RID-FACTORY".to_string(), "main_chain", Value::Null);
    slot.conversation_id = Some("CID-FACTORY".to_string());
    let slot = Arc::new(slot);

    let mut first = CmpStep::new("a", "", CmpStepTypeEnum::Single);
    first.set_node_name("节点A");
    first.set_node_instance_id("a_1");
    first.finish(true, None);
    slot.steps.lock().unwrap().push(first);

    let mut rollback = CmpStep::new("a", "", CmpStepTypeEnum::Single);
    rollback.set_node_name("节点A");
    rollback.finish_rollback(true, None);
    slot.rollback_steps.lock().unwrap().push(rollback);

    let mut success = LiteflowResponse::new_main_response(slot.clone());
    assert!(success.is_success());
    assert_eq!(success.get_execute_step_str(), "a[节点A]");
    assert_eq!(success.get_execute_step_str_with_instance_id(), "a[a_1]");
    assert_eq!(success.get_rollback_step_str(), "a[节点A]");
    assert!(
        success
            .get_rollback_step_str_with_time()
            .contains("a[节点A]<")
    );
    assert_eq!(success.get_rollback_steps().len(), 1);

    success.set_success(false);
    success.set_message("manual failure");
    success.set_code(Some("E-MANUAL".to_string()));
    success.set_cause(Some("manual cause".to_string()));
    success.set_chain_id("changed_chain");
    assert!(!success.is_success());
    assert_eq!(success.get_message(), "manual failure");
    assert_eq!(success.get_code(), Some("E-MANUAL"));
    assert_eq!(success.get_cause(), Some("manual cause"));
    assert_eq!(success.get_chain_id(), "changed_chain");

    slot.set_sub_exception("inner_chain", "inner failed");
    let inner = LiteflowResponse::new_inner_response("inner_chain", slot);
    assert!(!inner.is_success());
    assert_eq!(inner.get_message(), "inner failed");
    assert_eq!(inner.get_cause(), Some("inner failed"));
    // Java newInnerResponse 仍以当前 Slot 的主链 ID 填充响应。
    assert_eq!(inner.get_chain_id(), "main_chain");

    let initialization = LiteflowResponse::new_main_response_with_cause("init failed");
    assert!(!initialization.is_success());
    assert_eq!(initialization.get_cause(), Some("init failed"));
}
