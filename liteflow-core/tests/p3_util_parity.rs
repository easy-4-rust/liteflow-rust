use std::collections::HashSet;

use liteflow_core::{
    BoundedPriorityBlockingQueue, ConversationIdGenerator, ExecuteOption, FlowBus, JsonUtil,
    LimitQueue, SelectiveJavaEscaper, TupleOf2, TupleOf3, cmp,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

fn assert_java_conversation_id_shape(conversation_id: &str) {
    let (date, code) = conversation_id
        .split_once('_')
        .expect("conversationId 应包含日期与随机码分隔符");
    assert_eq!(date.len(), 8);
    assert!(date.bytes().all(|byte| byte.is_ascii_digit()));
    assert_eq!(code.len(), 12);
    assert!(
        code.bytes()
            .all(|byte| b"123456789ABCDEFGHIJKLMNPQRSTUVWXYZ".contains(&byte))
    );
}

#[test]
fn conversation_id_generator_matches_java_shape_and_random_space() {
    let ids: HashSet<String> = (0..64)
        .map(|_| ConversationIdGenerator::generate())
        .collect();
    assert_eq!(ids.len(), 64);
    for conversation_id in ids {
        assert_java_conversation_id_shape(&conversation_id);
    }
}

#[tokio::test]
async fn auto_conversation_id_is_wired_into_real_flow_execution() {
    let bus = FlowBus::new();
    bus.register(
        "capture",
        cmp(|context| async move {
            let conversation_id = context
                .conversation_id()
                .expect("auto conversationId 应写入 Slot");
            assert_java_conversation_id_shape(conversation_id);
            Ok(Value::Null)
        }),
    );
    bus.add_chain("conversation_chain", "THEN(capture)")
        .unwrap();

    let response = bus
        .execute_with_option(
            "conversation_chain",
            Value::Null,
            ExecuteOption::of().auto_conversation_id(),
        )
        .await;
    assert!(response.is_success(), "{}", response.message);
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct JsonBean {
    name: String,
}

#[test]
fn json_util_preserves_null_empty_list_and_error_semantics() {
    let bean = JsonBean {
        name: "liteflow".to_string(),
    };
    let json = JsonUtil::to_json_string(Some(&bean)).unwrap().unwrap();
    assert_eq!(
        JsonUtil::parse_object::<JsonBean>(&json).unwrap(),
        Some(bean)
    );
    assert_eq!(JsonUtil::to_json_string::<JsonBean>(None).unwrap(), None);
    assert_eq!(JsonUtil::parse_value("").unwrap(), None);
    assert!(JsonUtil::parse_list::<JsonBean>("").unwrap().is_empty());
    assert!(JsonUtil::parse_value("{invalid").is_err());
}

#[test]
fn limit_queue_offer_evicts_oldest_while_add_keeps_java_delegate_behavior() {
    let queue = LimitQueue::new(2);
    queue.offer("a");
    queue.offer("b");
    queue.offer("c");
    assert_eq!(queue.queue(), vec!["b", "c"]);
    assert_eq!(queue.peek(), Some("b"));

    // Java add() 直接委托 ConcurrentLinkedQueue，不触发 limit 淘汰。
    queue.add("d");
    assert_eq!(queue.len(), 3);
    assert_eq!(queue.poll(), Some("b"));
    assert!(queue.remove(&"d"));
    assert!(!queue.contains(&"d"));
}

#[test]
fn bounded_priority_queue_retains_highest_priority_window() {
    let queue = BoundedPriorityBlockingQueue::new(3);
    assert!(queue.add_all([5, 1, 3]));
    assert!(!queue.offer(7));
    assert!(queue.offer(2));

    assert_eq!(queue.len(), 3);
    assert_eq!(queue.to_list(), vec![1, 2, 3]);
    assert_eq!(queue.iterator().collect::<Vec<_>>(), vec![1, 2, 3]);
    assert_eq!((&queue).into_iter().collect::<Vec<_>>(), vec![1, 2, 3]);
}

#[test]
fn tuple_objects_support_java_style_independent_mutation() {
    let mut pair = TupleOf2::new("a", 1);
    pair.set_a("changed");
    pair.set_b(2);
    assert_eq!((pair.a(), pair.b()), (&"changed", &2));

    let mut triple = TupleOf3::new("a", 1, false);
    triple.set_a("changed");
    triple.set_b(2);
    triple.set_c(true);
    assert_eq!(
        (triple.a(), triple.b(), triple.c()),
        (&"changed", &2, &true)
    );
}

#[test]
fn selective_java_escaper_keeps_unicode_and_escapes_only_required_chars() {
    let source = "你好，\"世界\"！\n\t\\End\r\u{000c}\u{0008}";
    let escaped = SelectiveJavaEscaper::escape(Some(source)).unwrap();
    assert_eq!(escaped, "你好，\\\"世界\\\"！\\n\\t\\\\End\\r\\f\\b");
    assert_eq!(SelectiveJavaEscaper::escape(None), None);
}
