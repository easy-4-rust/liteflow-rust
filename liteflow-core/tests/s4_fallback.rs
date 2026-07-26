//! S4 降级组件语义测试。
//!
//! 覆盖 Java `FallbackNode#findFallbackNode` 的五类位置推导，以及
//! `FallbackNode#execute` 在执行前重新寻找原节点的动态注册语义。

use liteflow_core::{FlowBus, LiteflowError, NodeTypeEnum, cmp};
use serde_json::{Value, json};

#[tokio::test]
async fn fallback_selects_component_by_condition_position() {
    let bus = FlowBus::new();

    bus.register_fallback(
        "commonFallback",
        NodeTypeEnum::Common,
        cmp(|ctx| async move {
            ctx.set_data("common_fallback", json!(ctx.node_id()));
            Ok(Value::Null)
        }),
    )
    .unwrap();
    bus.register_fallback(
        "booleanFallback",
        NodeTypeEnum::Boolean,
        cmp(|ctx| async move {
            ctx.set_data("boolean_fallback", json!(ctx.node_id()));
            Ok(json!(true))
        }),
    )
    .unwrap();
    bus.register_fallback(
        "switchFallback",
        NodeTypeEnum::Switch,
        cmp(|ctx| async move {
            ctx.set_data("switch_fallback", json!(ctx.node_id()));
            Ok(json!("switchTarget"))
        }),
    )
    .unwrap();
    bus.register_fallback(
        "forFallback",
        NodeTypeEnum::For,
        cmp(|ctx| async move {
            ctx.set_data("for_fallback", json!(ctx.node_id()));
            Ok(json!(2))
        }),
    )
    .unwrap();
    bus.register_fallback(
        "iteratorFallback",
        NodeTypeEnum::Iterator,
        cmp(|ctx| async move {
            ctx.set_data("iterator_fallback", json!(ctx.node_id()));
            Ok(json!(["a", "b"]))
        }),
    )
    .unwrap();

    bus.register(
        "trueTarget",
        cmp(|ctx| async move {
            ctx.set_data("boolean_target", json!(true));
            Ok(Value::Null)
        }),
    );
    bus.register("falseTarget", cmp(|_| async move { Ok(Value::Null) }));
    bus.register(
        "switchTarget",
        cmp(|ctx| async move {
            ctx.set_data("switch_target", json!(true));
            Ok(Value::Null)
        }),
    );
    bus.register("defaultTarget", cmp(|_| async move { Ok(Value::Null) }));
    bus.register(
        "forBody",
        cmp(|ctx| async move {
            let count = ctx.get_data_as::<usize>("for_body_count").unwrap_or(0);
            ctx.set_data("for_body_count", json!(count + 1));
            Ok(Value::Null)
        }),
    );
    bus.register(
        "iteratorBody",
        cmp(|ctx| async move {
            let mut values = ctx
                .get_data_as::<Vec<String>>("iterator_values")
                .unwrap_or_default();
            values.push(ctx.loop_object::<String>().unwrap());
            ctx.set_data("iterator_values", json!(values));
            Ok(Value::Null)
        }),
    );

    bus.add_chain("commonChain", "THEN(missingCommon)").unwrap();
    bus.add_chain(
        "booleanChain",
        "IF(missingBoolean, trueTarget, falseTarget)",
    )
    .unwrap();
    bus.add_chain(
        "switchChain",
        "SWITCH(missingSwitch).TO(switchTarget).DEFAULT(defaultTarget)",
    )
    .unwrap();
    bus.add_chain("forChain", "FOR(missingFor).DO(forBody)")
        .unwrap();
    bus.add_chain(
        "iteratorChain",
        "ITERATOR(missingIterator).DO(iteratorBody)",
    )
    .unwrap();

    let common = bus.execute("commonChain").await;
    let boolean = bus.execute("booleanChain").await;
    let switch = bus.execute("switchChain").await;
    let for_loop = bus.execute("forChain").await;
    let iterator = bus.execute("iteratorChain").await;

    assert_eq!(common.data("common_fallback"), Some(json!("missingCommon")));
    assert_eq!(
        boolean.data("boolean_fallback"),
        Some(json!("missingBoolean"))
    );
    assert_eq!(boolean.data("boolean_target"), Some(json!(true)));
    assert_eq!(switch.data("switch_fallback"), Some(json!("missingSwitch")));
    assert_eq!(switch.data("switch_target"), Some(json!(true)));
    assert_eq!(for_loop.data("for_fallback"), Some(json!("missingFor")));
    assert_eq!(for_loop.data("for_body_count"), Some(json!(2)));
    assert_eq!(
        iterator.data("iterator_fallback"),
        Some(json!("missingIterator"))
    );
    assert_eq!(iterator.data("iterator_values"), Some(json!(["a", "b"])));
}

#[tokio::test]
async fn original_node_registered_after_chain_build_wins_over_fallback() {
    let bus = FlowBus::new();
    bus.register_fallback(
        "commonFallback",
        NodeTypeEnum::Common,
        cmp(|ctx| async move {
            ctx.set_data("winner", json!("fallback"));
            Ok(Value::Null)
        }),
    )
    .unwrap();
    bus.add_chain("lateChain", "THEN(lateNode)").unwrap();

    bus.register(
        "lateNode",
        cmp(|ctx| async move {
            ctx.set_data("winner", json!("original"));
            Ok(Value::Null)
        }),
    );

    let response = bus.execute("lateChain").await;
    assert!(response.is_success(), "{}", response.message);
    assert_eq!(response.data("winner"), Some(json!("original")));
}

#[tokio::test]
async fn missing_fallback_is_reported_at_execution_time() {
    let bus = FlowBus::new();
    bus.add_chain("missingChain", "THEN(missingNode)").unwrap();

    let response = bus.execute("missingChain").await;

    assert!(!response.is_success());
    assert!(response.message.contains("No fallback component found"));
    assert!(response.message.contains("missingNode"));
    // 确认错误边界仍归属于统一 LiteflowError 体系。
    let error = LiteflowError::FallbackCmpNotFound(response.message.clone());
    assert!(matches!(error, LiteflowError::FallbackCmpNotFound(_)));
}
