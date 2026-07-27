//! LiteFlowChainELBuilder 的 Java v2.16 状态机与两阶段构建验收。

use std::sync::Arc;

use liteflow_core::builder::el::LiteFlowChainELBuilder;
use liteflow_core::{
    FlowBus, LiteflowConfig, LiteflowConfigGetter, LiteflowError, NodeTypeEnum, ParseModeEnum, cmp,
};
use serde_json::Value;

/// 验证 Java 命名 setter、立即编译、延迟编译、依赖顺序和路由类型约束。
#[allow(deprecated)]
#[tokio::test]
async fn java_builder_state_and_two_phase_compilation_are_aligned() {
    LiteflowConfigGetter::clean();
    let bus = FlowBus::new();
    bus.add_node(
        "body",
        None,
        NodeTypeEnum::Common,
        Arc::new(cmp(|ctx| async move {
            ctx.set_data("executed", Value::Bool(true));
            Ok(Value::Null)
        })),
    )
    .unwrap();
    bus.add_node(
        "route_bool",
        None,
        NodeTypeEnum::Boolean,
        Arc::new(cmp(|_| async { Ok(Value::Bool(true)) })),
    )
    .unwrap();
    bus.add_node(
        "route_common",
        None,
        NodeTypeEnum::Common,
        Arc::new(cmp(|_| async { Ok(Value::Null) })),
    )
    .unwrap();

    let builder = LiteFlowChainELBuilder::create_chain(bus.clone());
    builder
        .set_chain_id("immediate")
        .set_namespace("orders")
        .set_thread_pool_executor_class("fast-pool")
        .set_route("route_bool");
    builder.set_el(" THEN( body ) ").unwrap();
    {
        let chain = builder.get_chain();
        assert_eq!(chain.get_chain_id(), "immediate");
        assert_eq!(chain.get_namespace(), "orders");
        assert_eq!(chain.get_thread_pool_executor_class(), Some("fast-pool"));
        assert_eq!(chain.get_route_el(), Some("route_bool"));
        assert!(chain.get_el_md5().is_some());
    }
    builder.build().unwrap();

    let immediate = bus
        .get_chain_map()
        .remove("immediate")
        .expect("build 应把完整 Chain 注册到 FlowBus");
    assert!(immediate.is_compiled());
    assert!(immediate.get_route_item().is_some());
    let responses = bus
        .execute_route_chain(Some("orders"), Value::Null)
        .await
        .expect("route Chain 应成功匹配并执行");
    assert_eq!(responses.len(), 1);
    let response = &responses[0];
    assert!(response.is_success(), "{}", response.message);
    assert_eq!(response.data("executed"), Some(Value::Bool(true)));

    // setChainName 沿用现有 Chain；setChainId 则按 Java 语义标记为待重新编译。
    let existing_by_name = LiteFlowChainELBuilder::create_chain(bus.clone());
    existing_by_name.set_chain_name("immediate");
    assert!(existing_by_name.get_chain().is_compiled());
    let existing_by_id = LiteFlowChainELBuilder::create_chain(bus.clone());
    existing_by_id.set_chain_id("immediate");
    assert!(!existing_by_id.get_chain().is_compiled());

    // 空白 namespace 回退 default；空白 route 不覆盖已有 route。
    let copied = LiteFlowChainELBuilder::from_chain(bus.clone(), (*immediate).clone());
    copied.set_namespace("").set_route("   ");
    assert_eq!(copied.get_chain().get_namespace(), "default");
    assert_eq!(copied.get_chain().get_route_el(), Some("route_bool"));

    // PARSE_ONE_ON_FIRST_EXEC 仅预装载；显式 buildUnCompileChain 会先编译依赖。
    let mut lazy_config = LiteflowConfig::default();
    lazy_config.set_parse_mode(ParseModeEnum::ParseOneOnFirstExec);
    LiteflowConfigGetter::set_liteflow_config(lazy_config);

    let child = LiteFlowChainELBuilder::create_chain(bus.clone());
    child.set_chain_id("lazy_child");
    child.set_el("THEN(body)").unwrap();
    child.build().unwrap();

    let parent = LiteFlowChainELBuilder::create_chain(bus.clone());
    parent.set_chain_id("lazy_parent");
    parent.set_el("THEN(lazy_child)").unwrap();
    parent.build().unwrap();
    let lazy_snapshot = bus.get_chain_map();
    assert!(!lazy_snapshot["lazy_child"].is_compiled());
    assert!(!lazy_snapshot["lazy_parent"].is_compiled());

    LiteFlowChainELBuilder::build_un_compile_chain(&bus, &lazy_snapshot["lazy_parent"]).unwrap();
    let compiled_snapshot = bus.get_chain_map();
    assert!(compiled_snapshot["lazy_child"].is_compiled());
    assert!(compiled_snapshot["lazy_parent"].is_compiled());
    assert!(bus.execute("lazy_parent").await.is_success());

    // 递归依赖必须明确失败，不能无限递归或留下伪编译状态。
    let cycle_a = LiteFlowChainELBuilder::create_chain(bus.clone());
    cycle_a.set_chain_id("cycle_a");
    cycle_a.set_el("THEN(cycle_b)").unwrap();
    cycle_a.build().unwrap();
    let cycle_b = LiteFlowChainELBuilder::create_chain(bus.clone());
    cycle_b.set_chain_id("cycle_b");
    cycle_b.set_el("THEN(cycle_a)").unwrap();
    cycle_b.build().unwrap();
    let cycles = bus.get_chain_map();
    assert!(matches!(
        LiteFlowChainELBuilder::build_un_compile_chain(&bus, &cycles["cycle_a"]),
        Err(LiteflowError::CyclicDependency(message))
            if message.contains("cycle_a")
    ));

    LiteflowConfigGetter::clean();

    // route 只允许布尔 Node、AND/OR/NOT；普通 Condition 和 Common Node 均拒绝。
    let invalid_condition_route = LiteFlowChainELBuilder::create_chain(bus.clone());
    invalid_condition_route.set_chain_id("invalid_condition_route");
    invalid_condition_route.set_el("THEN(body)").unwrap();
    invalid_condition_route.set_route("THEN(route_bool)");
    assert!(matches!(
        invalid_condition_route.build(),
        Err(LiteflowError::RouteELInvalid(message))
            if message == "the route EL can only be a boolean node, or an AND or OR expression."
    ));

    let invalid_node_route = LiteFlowChainELBuilder::create_chain(bus.clone());
    invalid_node_route.set_chain_id("invalid_node_route");
    invalid_node_route.set_el("THEN(body)").unwrap();
    invalid_node_route.set_route("route_common");
    assert!(matches!(
        invalid_node_route.build(),
        Err(LiteflowError::Parse(message))
            if message == "The node[route_common] must be boolean type Node."
    ));

    LiteflowConfigGetter::clean();
}
