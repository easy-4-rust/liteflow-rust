//! QLExpress Java 对象级缓存与编译契约测试。

use std::sync::Arc;

use liteflow_core::FlowBus;
use liteflow_core::el::NodeRef;
use liteflow_core::script::{ScriptExecutor, ScriptExecutorFactory, ScriptKind};
use liteflow_core::slot::{CmpContext, DataBus, Frame, Slot};
use liteflow_script_qlexpress::QlExpressScriptExecutor;
use serde_json::json;

#[test]
fn executor_compiles_caches_unloads_and_rejects_invalid_qlexpress() {
    let executor = QlExpressScriptExecutor::new();
    let script = r#"
        // Java QLExpress 支持单行注释与单词形式的逻辑运算符。
        count = defaultContext.getData("count");
        if (count > 100 and not defaultContext.hasData("blocked")) {
            return "a";
        } else {
            return "b";
        }
    "#;

    let validation = executor.validate_with_ex(script);
    assert!(
        validation.is_success(),
        "真实 qlexpress-rust 编译失败：{:?}",
        validation.cause()
    );
    assert!(!executor.validate("if (count >) { return true; }"));

    executor.load("switch_node", script).unwrap();
    executor.load("for_node", "return 3;").unwrap();
    assert_eq!(
        executor.node_ids().unwrap(),
        vec!["for_node".to_string(), "switch_node".to_string()]
    );

    executor.unload("switch_node").unwrap();
    assert_eq!(executor.node_ids().unwrap(), vec!["for_node".to_string()]);
    executor.clean_cache().unwrap();
    assert!(executor.node_ids().unwrap().is_empty());
}

#[tokio::test]
async fn published_qlexpress_executes_through_real_flow_bus() {
    QlExpressScriptExecutor::register().unwrap();
    assert!(ScriptExecutorFactory::contains("qlexpress"));

    let flow_bus = FlowBus::new();
    flow_bus
        .register_script_typed(
            "qlexpress_case",
            "qlexpress",
            ScriptKind::Common,
            r#"
                left = 20;
                right = 22;
                defaultContext.setData("qlexpress_result", left + right);
            "#,
        )
        .unwrap();
    flow_bus
        .add_chain("qlexpress_case_chain", "THEN(qlexpress_case)")
        .unwrap();

    // 从 FlowBus 进入脚本组件、真实 QLExpress QVM，再把结果写回同一 Slot。
    let response = flow_bus.execute("qlexpress_case_chain").await;
    assert!(response.is_success(), "执行失败：{}", response.message);
    assert_eq!(response.data_as::<i64>("qlexpress_result"), Some(42));
}

/// 验证 Java `ScriptExecutor#bindParam` 的完整绑定表进入真实 QLExpress QVM。
///
/// 该用例覆盖 serde 上下文 Bean、隐式子流程请求、循环元数据和当前 Chain，
/// 防止 QLExpress 适配层仅手工绑定少量字段后与其他脚本执行器漂移。
#[test]
fn published_qlexpress_consumes_complete_java_bind_param_context() {
    let slot = Arc::new(Slot::new(
        "qlexpress-bind-request".to_string(),
        "main-chain",
        json!({"order": 7}),
    ));
    slot.insert_context_bean(
        "profile",
        Arc::new(json!({
            "customer": "Ada",
            "level": 3,
            "unsignedMax": u64::MAX
        })),
    );
    slot.set_chain_req_data("sub-chain", json!({"sub": 9}));
    let slot_index = DataBus::offer_slot(Arc::clone(&slot));
    let mut node = NodeRef::new("qlexpress-bind-node");
    node.tag = Some("blue".to_string());
    node.data = Some(r#"{"limit":2}"#.to_string());
    let context = CmpContext {
        inner: slot,
        node,
        frame: Frame::root()
            .with_current_chain_id("sub-chain")
            .push(4, Some(json!({"sku": "A"}))),
    };
    let executor = QlExpressScriptExecutor::new();
    executor
        .load(
            "qlexpress-bind-node",
            r#"
                defaultContext.setData("customer", profile.get("customer"));
                defaultContext.setData("unsigned_max", profile.get("unsignedMax"));
                defaultContext.setData("sub", _meta.get("subRequestData").get("sub"));
                defaultContext.setData("chain", _meta.get("currChainId"));
                defaultContext.setData("loop", _meta.get("loopObject").get("sku"));
                defaultContext.setData("slot", _meta.get("slotIndex"));
                return 42;
            "#,
        )
        .unwrap();

    let result = executor.execute("qlexpress-bind-node", &context).unwrap();

    assert_eq!(result, json!(42));
    assert_eq!(context.get_data("customer"), Some(json!("Ada")));
    assert_eq!(context.get_data("unsigned_max"), Some(json!(u64::MAX)));
    assert_eq!(context.get_data("sub"), Some(json!(9)));
    assert_eq!(context.get_data("chain"), Some(json!("sub-chain")));
    assert_eq!(context.get_data("loop"), Some(json!("A")));
    assert_eq!(context.get_data("slot"), Some(json!(slot_index)));
    assert!(DataBus::release_slot(slot_index));
}
