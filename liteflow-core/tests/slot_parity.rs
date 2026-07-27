//! Java Slot 对等数据职责与并发队列测试。

use std::sync::Arc;

use liteflow_core::flow::element::chain::Chain;
use liteflow_core::flow::element::condition::Condition;
use liteflow_core::flow::element::condition::then_condition::ThenCondition;
use liteflow_core::{CmpStep, CmpStepTypeEnum, ConditionTypeEnum, Frame, Slot};
use serde_json::json;

/// 验证节点数据、响应数据、子链请求和私有传递队列的真实读写语义。
#[test]
fn slot_data_and_queue_methods_preserve_java_semantics() {
    let slot = Slot::new("request-1".to_string(), "main", json!({"order": 1}));

    slot.set_input("a", json!({"input": 1}));
    slot.set_output("a", json!({"output": 2}));
    slot.set_response_data(json!({"accepted": true}));
    slot.set_chain_req_data("sub", json!("single"));
    slot.set_chain_req_data2_queue("sub", json!(1));
    slot.set_chain_req_data2_queue("sub", json!(2));
    slot.set_private_delivery_data("b", json!("first"));
    slot.set_private_delivery_data("b", json!("second"));

    assert_eq!(slot.get_input("a"), Some(json!({"input": 1})));
    assert_eq!(slot.get_output("a"), Some(json!({"output": 2})));
    assert_eq!(slot.get_response_data(), Some(json!({"accepted": true})));
    assert_eq!(slot.get_chain_req_data("sub"), Some(json!("single")));
    assert_eq!(slot.get_chain_req_data_from_queue("sub"), Some(json!(1)));
    assert_eq!(slot.get_chain_req_data_from_queue("sub"), Some(json!(2)));
    assert_eq!(slot.get_chain_req_data_from_queue("sub"), None);
    assert_eq!(
        slot.get_private_delivery_queue("b")
            .expect("私有队列应存在")
            .len(),
        2
    );
    assert_eq!(slot.get_private_delivery_data("b"), Some(json!("first")));
    assert_eq!(slot.get_private_delivery_data("b"), Some(json!("second")));
}

/// 验证 Chain、路由、步骤、请求标识和上下文 Bean 的 Java 命名入口。
#[test]
#[allow(deprecated)]
fn slot_execution_metadata_methods_are_real() {
    let mut slot = Slot::new(String::new(), String::new(), json!(null));
    slot.set_chain_id("main");
    slot.set_chain_name("ignored");
    slot.put_request_id("request-2");
    slot.set_conversation_id("conversation-1");
    slot.set_route_result(true);
    slot.insert_context_bean("number", Arc::new(7_u32));

    let chain = Arc::new(Chain::new("sub", Vec::new()));
    slot.add_chain_instance(chain.clone());

    let mut execute_step = CmpStep::new("a", "", CmpStepTypeEnum::Single);
    execute_step.node_instance_id = Some("a-0".to_string());
    execute_step.finish(true, None);
    slot.add_step(execute_step);

    let mut rollback_step = CmpStep::new("a", "", CmpStepTypeEnum::Single);
    rollback_step.finish_rollback(true, None);
    slot.add_rollback_step(rollback_step);

    assert_eq!(slot.get_chain_id(), "main");
    assert_eq!(slot.get_chain_name(), "main");
    assert_eq!(slot.get_request_id(), "request-2");
    assert_eq!(slot.get_conversation_id(), Some("conversation-1"));
    assert_eq!(slot.get_route_result(), Some(true));
    assert_eq!(
        slot.get_current_chain_instance("sub")
            .expect("Chain 应已登记")
            .id,
        chain.id
    );
    assert_eq!(slot.get_context_bean::<u32>("number").as_deref(), Some(&7));
    assert_eq!(slot.get_context_bean_list().len(), 1);
    assert_eq!(slot.get_execute_steps().len(), 1);
    assert_eq!(slot.get_rollback_steps().len(), 1);
    assert_eq!(slot.get_execute_step_str(false), "a");
    assert!(slot.get_execute_step_str_with_instance_id().contains("a-0"));
    assert_eq!(slot.get_rollback_step_str(false), "a");
}

/// 验证 Java ThreadLocal 状态映射为 Frame 后，父任务与子任务互不污染。
#[test]
fn slot_task_local_results_and_condition_stack_are_isolated() {
    let slot = Slot::new("request-3".to_string(), "main", json!(null));
    let mut parent = Frame::root();

    slot.set_switch_result(&mut parent, "switch", json!("parent"));
    slot.set_if_result(&mut parent, "if", true);
    slot.set_and_or_result(&mut parent, "and_or", true);
    slot.set_not_result(&mut parent, "not", false);
    slot.set_for_result(&mut parent, "for", 3);
    slot.set_while_result(&mut parent, "while", true);
    slot.set_break_result(&mut parent, "break", false);
    slot.set_iterator_result(&mut parent, "iterator", [json!(1), json!(2)]);

    let condition: Arc<dyn Condition> = Arc::new(ThenCondition::new());
    slot.push_condition(&mut parent, condition);

    let mut child = parent.clone();
    slot.set_switch_result(&mut child, "switch", json!("child"));
    slot.set_if_result(&mut child, "if", false);
    slot.set_iterator_result(&mut child, "iterator", [json!(9)]);
    assert!(slot.pop_condition(&mut child).is_some());

    assert_eq!(
        slot.get_switch_result(&parent, "switch"),
        Some(json!("parent"))
    );
    assert_eq!(slot.get_if_result(&parent, "if"), Some(true));
    assert_eq!(slot.get_and_or_result(&parent, "and_or"), Some(true));
    assert_eq!(slot.get_not_result(&parent, "not"), Some(false));
    assert_eq!(slot.get_for_result(&parent, "for"), Some(3));
    assert_eq!(slot.get_while_result(&parent, "while"), Some(true));
    assert_eq!(slot.get_break_result(&parent, "break"), Some(false));
    assert_eq!(
        slot.get_iterator_result(&parent, "iterator")
            .expect("父任务迭代器应存在")
            .collect::<Vec<_>>(),
        vec![json!(1), json!(2)]
    );
    assert_eq!(
        slot.get_iterator_result(&child, "iterator")
            .expect("子任务迭代器应存在")
            .collect::<Vec<_>>(),
        vec![json!(9)]
    );
    assert_eq!(
        slot.get_current_condition(&parent)
            .expect("父任务 Condition 应仍在栈中")
            .condition_type(),
        ConditionTypeEnum::Then
    );
    assert_eq!(slot.get_condition_stack(&parent).len(), 1);
    assert!(slot.get_current_condition(&child).is_none());
}
