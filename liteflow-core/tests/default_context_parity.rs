//! Java `DefaultContext` 并发键值和空值拒绝语义回归测试。

use std::sync::Arc;
use std::thread;

use liteflow_core::DefaultContext;
use serde_json::json;

/// 验证真实并发存储、拥有型快照和 Java null 拒绝行为。
#[test]
fn concurrent_data_map_preserves_java_default_context_semantics() {
    let context = Arc::new(DefaultContext::new());
    let workers = (0..8)
        .map(|index| {
            let context = Arc::clone(&context);
            thread::spawn(move || {
                context
                    .set_data(format!("key-{index}"), json!(index))
                    .expect("非空 serde 值应成功写入");
            })
        })
        .collect::<Vec<_>>();

    for worker in workers {
        worker.join().expect("并发写入线程不应失败");
    }

    assert!(context.has_data("key-3"));
    assert_eq!(context.get_data("key-3"), Some(json!(3)));

    let snapshot = context.get_data_map();
    assert_eq!(snapshot.len(), 8);
    context
        .set_data("later", json!(true))
        .expect("后续非空值应成功写入");
    assert!(!snapshot.contains_key("later"));

    let error = context
        .set_data("null", serde_json::Value::Null)
        .expect_err("Java ConcurrentHashMap 语义必须拒绝 null");
    assert_eq!(error.to_string(), "data can't accept null param");
    assert!(!context.has_data("null"));
}
