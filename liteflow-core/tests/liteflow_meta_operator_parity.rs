//! LiteflowMetaOperator 的 Java v2.16.0 对象级语义验收。

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use liteflow_core::flow::element::chain::Chain;
use liteflow_core::meta::LiteflowMetaOperator;
use liteflow_core::{FlowBus, LiteflowError, NodeTypeEnum, cmp};
use serde_json::Value;

fn chain_with_el(el: &str) -> Chain {
    let mut chain = Chain::new("metadata-only", Vec::new());
    chain.set_el(el);
    chain
}

/// 验证 getNodes(Executable) 对所有 Java Condition 形态均按出现顺序递归提取节点。
///
/// 对应 Java: `LiteflowMetaOperator#getNodes(Executable)`。
#[test]
fn get_nodes_from_chain_traverses_every_java_condition_shape_in_order() {
    let cases = [
        ("true", Vec::<&str>::new()),
        ("THEN(a,b)", vec!["a", "b"]),
        ("AND(a,b)", vec!["a", "b"]),
        ("OR(a,b)", vec!["a", "b"]),
        ("WHEN(a,b)", vec!["a", "b"]),
        ("IF(p,a).ELIF(q,b).ELSE(c)", vec!["p", "a", "q", "b", "c"]),
        (
            "SWITCH(selector).TO(a,b).DEFAULT(c)",
            vec!["selector", "a", "b", "c"],
        ),
        (
            "FOR(counter).DO(body).BREAK(stop)",
            vec!["counter", "body", "stop"],
        ),
        ("FOR(2).DO(body).BREAK(stop)", vec!["body", "stop"]),
        ("WHILE(p).DO(body).BREAK(stop)", vec!["p", "body", "stop"]),
        (
            "ITERATOR(items).DO(body).BREAK(stop)",
            vec!["items", "body", "stop"],
        ),
        ("CATCH(body).DO(handler)", vec!["body", "handler"]),
        ("NOT(p)", vec!["p"]),
        ("PRE(a)", vec!["a"]),
        ("FINALLY(a)", vec!["a"]),
        (r#"THEN(a).tag("condition")"#, vec!["a"]),
    ];

    for (el, expected) in cases {
        let actual = LiteflowMetaOperator::get_nodes_from_chain(&chain_with_el(el))
            .into_iter()
            .map(|node| node.id)
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "EL={el}");
    }

    let without_el = Chain::new("without-el", Vec::new());
    assert!(LiteflowMetaOperator::get_nodes_from_chain(&without_el).is_empty());
    assert!(
        LiteflowMetaOperator::get_nodes_from_chain(&chain_with_el("THEN(")).is_empty(),
        "Java 无可执行 Condition 时返回空列表，元数据查询不传播解析异常"
    );
}

/// 验证 Chain 查询、route 重载、批量卸载和全量刷新都作用于同一个 FlowBus。
///
/// 对应 Java: `LiteflowMetaOperator` 的 Chain 相关全部公共入口。
#[tokio::test]
async fn chain_operations_share_the_real_flow_bus_and_route_overload() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    bus.register("b", cmp(|_| async { Ok(Value::Null) }));
    bus.add_node(
        "route",
        None,
        NodeTypeEnum::Boolean,
        Arc::new(cmp(|_| async { Ok(Value::Bool(true)) })),
    )
    .unwrap();
    bus.add_chain("first", "THEN(a,a)").unwrap();
    bus.add_chain("second", "THEN(b)").unwrap();

    let reload_count = Arc::new(AtomicUsize::new(0));
    let count = reload_count.clone();
    let metadata = LiteflowMetaOperator::new(bus.clone()).with_reload_all(Arc::new(move || {
        count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }));

    metadata.reload_all_chain().unwrap();
    assert_eq!(reload_count.load(Ordering::SeqCst), 1);
    assert_eq!(metadata.get_chain("first").unwrap().get_chain_id(), "first");
    assert_eq!(metadata.get_nodes("first").len(), 2);
    assert_eq!(metadata.get_nodes_by_id("first", "a").len(), 2);
    assert_eq!(metadata.get_node_by_index("first", "a", 1).unwrap().id, "a");
    assert!(metadata.get_node_by_index("first", "a", 2).is_none());
    assert_eq!(metadata.get_nodes_in_all_chain("a").len(), 2);
    assert_eq!(metadata.get_chains_contains_node_id("a").len(), 1);

    metadata
        .reload_one_chain_with_route("routed", "THEN(a)", "route")
        .unwrap();
    assert!(
        metadata
            .get_chain("routed")
            .unwrap()
            .get_route_item()
            .is_some()
    );
    metadata.reload_one_chain("routed", "THEN(b)").unwrap();
    assert!(
        metadata
            .get_chain("routed")
            .unwrap()
            .get_route_item()
            .is_some(),
        "Java 两参数 reloadOneChain 应保留已有 route"
    );

    metadata.remove_chains(["first", "second"]);
    assert!(metadata.get_chain("first").is_none());
    assert!(metadata.get_chain("second").is_none());
    metadata.remove_chain("missing");
}

/// 验证未绑定 FlowExecutor 刷新入口时保留明确错误，而非静默成功。
///
/// Java 静态入口会要求 `FlowExecutorHolder` 已初始化；Rust 用显式回调表达同一前置条件。
#[test]
fn reload_all_requires_configured_rule_source() {
    let error = LiteflowMetaOperator::new(FlowBus::new())
        .reload_all_chain()
        .expect_err("缺少全量规则刷新入口必须失败");
    assert!(matches!(error, LiteflowError::FlowExecutorNotInit(_)));
}
