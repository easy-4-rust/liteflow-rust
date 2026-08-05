//! 集合/帧/上下文完整 API 补测（批次 L）。
//!
//! 覆盖 Java 对偶对象的全部公开入口：
//! - `CopyOnWriteHashMap`（Java CopyOnWriteHashMap 语义）
//! - `LimitQueue`（Java LimitQueue 语义）
//! - `Frame` 的节点结果/迭代器结果/线程池/运行时 ID 全 API
//! - `CmpContext` 的 data/attachment/bean 全 API
//! - `Slot` 的 Java 命名 setter 全 API

use liteflow_core::slot::{CmpContext, Frame, Slot};
use liteflow_core::util::{CopyOnWriteHashMap, LimitQueue};
use liteflow_core::{NodeRef, cmp};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;

/// CopyOnWriteHashMap 的 Java 语义：快照读、整体替换写。
#[test]
fn copy_on_write_hash_map_full_api() {
    let mut map = HashMap::new();
    map.insert("a".to_string(), 1);
    map.insert("b".to_string(), 2);
    let cow = CopyOnWriteHashMap::new(map);

    assert_eq!(cow.len(), 2);
    assert_eq!(cow.size(), 2);
    assert!(!cow.is_empty());
    assert!(cow.contains_key(&"a".to_string()));
    assert!(cow.contains_value(&1));
    assert_eq!(cow.get(&"a".to_string()), Some(1));
    assert_eq!(cow.get(&"missing".to_string()), None);
    assert!(cow.key_set().contains("a"));
    assert!(cow.values().contains(&2));
    assert!(cow.entry_set().contains(&("a".to_string(), 1)));

    let modified = cow.clone();
    modified.insert("c".to_string(), 3);
    modified.put("a".to_string(), 10);
    assert_eq!(modified.len(), 3);
    assert_eq!(modified.get(&"a".to_string()), Some(10));
    modified.remove(&"b".to_string());
    assert!(!modified.contains_key(&"b".to_string()));

    let mut extra = HashMap::new();
    extra.insert("d".to_string(), 4);
    modified.put_all(&extra);
    assert_eq!(modified.get(&"d".to_string()), Some(4));
    modified.extend(vec![("e".to_string(), 5)]);
    assert_eq!(modified.get(&"e".to_string()), Some(5));

    let empty: CopyOnWriteHashMap<String, i32> = CopyOnWriteHashMap::new(HashMap::new());
    assert!(empty.is_empty());
    assert_eq!(empty.snapshot().len(), 0);
    assert!(!empty.to_string().is_empty());
}

/// LimitQueue 的 Java 语义：有界队列、淘汰头部。
#[test]
fn limit_queue_full_api() {
    let queue = LimitQueue::new(2);
    assert_eq!(queue.limit(), 2);
    assert_eq!(queue.get_limit(), 2);
    assert!(queue.is_empty());
    assert_eq!(queue.len(), 0);
    assert_eq!(queue.size(), 0);

    assert!(queue.offer(1));
    assert!(queue.offer(2));
    // 超限后淘汰头部（Java LimitQueue 语义：满时移除最早元素）
    assert!(queue.offer(3));
    assert_eq!(queue.len(), 2);
    assert_eq!(queue.peek(), Some(2));

    assert_eq!(queue.poll(), Some(2));
    assert_eq!(queue.element(), Some(3));
    assert_eq!(queue.poll(), Some(3));
    assert!(queue.is_empty());

    queue.add(5);
    assert!(queue.add_all(vec![6, 7, 8]));
    // addAll 直接追加（Java extend 语义），offer 才会淘汰
    assert_eq!(queue.len(), 4);
    assert!(queue.queue().contains(&7));
    assert!(queue.queue().contains(&8));

    queue.clear();
    assert!(queue.is_empty());
    assert_eq!(queue.poll(), None);
    assert_eq!(queue.element(), None);
}

/// Frame 的运行时 ID 与线程池元数据 API。
#[test]
fn frame_runtime_and_metadata_apis() {
    let mut frame = Frame::root().with_runtime_id(42);
    assert_eq!(frame.runtime_id(), Some(42));
    frame = frame.with_chain_thread_pool(Some("pool-x"));
    assert_eq!(frame.chain_thread_pool(), Some("pool-x"));
    assert_eq!(frame.condition_thread_pool(), None);
    assert!(frame.switch_target_list().is_empty());
}

/// CmpContext 的 data/attachment/bean API。
#[tokio::test]
async fn cmp_context_data_and_attachment_apis() {
    let slot = Arc::new(Slot::new("RID-CTX".to_string(), "main", Value::Null));
    let context = CmpContext {
        inner: slot.clone(),
        node: NodeRef::new("ctx-node"),
        frame: Frame::root(),
    };

    assert!(context.get_data("k").is_none());
    context.set_data("k", json!({"v": 1}));
    assert_eq!(context.get_data("k"), Some(json!({"v": 1})));
    assert!(
        context
            .get_data_as::<serde_json::Map<String, Value>>("k")
            .is_some()
    );
    assert_eq!(
        context.request_data::<serde_json::Value>(),
        Some(Value::Null)
    );

    context.set_attachment("att", 7_u32);
    assert_eq!(context.get_attachment::<u32>("att").map(|v| *v), Some(7));

    let bean: Arc<dyn std::any::Any + Send + Sync> = Arc::new(String::from("bean-value"));
    context.inner.insert_context_bean("bean", bean);
    assert!(context.bean::<String>("bean").is_some());
    assert!(context.bean::<String>("missing").is_none());
}

/// Slot 的 Java 命名 setter/getter 全 API。
///
/// `set_chain_name`/`get_chain_name` 是 Java 兼容的废弃入口，按 Java 语义验证。
#[test]
#[allow(deprecated)]
fn slot_java_named_setters_full_api() {
    let mut slot = Slot::new("RID-SLOT".to_string(), "main", Value::Null);
    // Java 语义：已有非空 chainId 时 setChainId 不覆盖
    slot.set_chain_id("chain-b");
    assert_eq!(slot.get_chain_id(), "main");
    let mut empty = Slot::new("RID-EMPTY".to_string(), String::new(), Value::Null);
    // setChainName 是 setChainId 的废弃兼容入口（Rust 实现与 Java 一致）
    empty.set_chain_name("链B");
    assert_eq!(empty.get_chain_name(), "链B");
    // 已非空时 setChainId 不覆盖
    empty.set_chain_id("chain-b");
    assert_eq!(empty.get_chain_id(), "链B");
    empty.put_request_id("RID-NEW");
    assert_eq!(empty.get_request_id(), "RID-NEW");

    // Java Slot 的条件结果存储辅助方法（基于 Frame）
    let mut frame = Frame::root();
    slot.set_and_or_result(&mut frame, "ao", true);
    assert_eq!(slot.get_and_or_result(&frame, "ao"), Some(true));
    slot.set_break_result(&mut frame, "brk", true);
    assert_eq!(slot.get_break_result(&frame, "brk"), Some(true));
    slot.set_switch_result(&mut frame, "sw", json!("target"));
    assert_eq!(slot.get_switch_result(&frame, "sw"), Some(json!("target")));
    slot.set_not_result(&mut frame, "nt", false);
    assert_eq!(slot.get_not_result(&frame, "nt"), Some(false));
}

/// Slot 上下文 Bean 检索 API。
#[tokio::test]
async fn slot_context_bean_apis() {
    let slot = Slot::new("RID-BEAN".to_string(), "main", Value::Null);
    slot.insert_context_bean("first", Arc::new(1_u32));
    slot.insert_context_bean("second", Arc::new(2_u32));
    assert_eq!(slot.get_context_bean::<u32>("first").map(|v| *v), Some(1));
    assert_eq!(
        slot.get_context_bean_by_type::<u32>().map(|v| *v),
        Some(1),
        "首个注册的 u32 Bean"
    );
    assert_eq!(slot.get_context_bean_list().len(), 2);

    let component = Arc::new(cmp(|_| async { Ok(Value::Null) }));
    let _ = component;
}
