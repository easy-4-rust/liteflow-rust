//! LiteFlowChainELBuilder 的 Java v2.16 状态机与两阶段构建验收。

use std::sync::Arc;

use liteflow_core::builder::el::LiteFlowChainELBuilder;
use liteflow_core::flow::element::chain::Chain;
use liteflow_core::{
    CmpContext, FlowBus, LiteflowConfig, LiteflowConfigGetter, LiteflowError, NodeTypeEnum,
    ParseModeEnum, cmp, parse_el,
};
use serde_json::Value;

struct NoProcessDeclComponent;

#[async_trait::async_trait]
impl liteflow_core::core::DeclComponent for NoProcessDeclComponent {
    async fn call(&self, method: &str, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Err(LiteflowError::Custom(format!(
            "unexpected declarative method: {method}"
        )))
    }

    fn has_method(&self, _method: &str) -> bool {
        false
    }
}

struct ProcessDeclComponent;

#[async_trait::async_trait]
impl liteflow_core::core::DeclComponent for ProcessDeclComponent {
    async fn call(&self, method: &str, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        assert_eq!(method, "process");
        Ok(Value::Null)
    }

    fn has_method(&self, method: &str) -> bool {
        method == "process"
    }
}

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
    let new_by_name = LiteFlowChainELBuilder::create_chain(bus.clone());
    new_by_name.set_chain_name("new_by_name");
    assert_eq!(new_by_name.get_chain().get_chain_id(), "new_by_name");
    let existing_by_id = LiteFlowChainELBuilder::create_chain(bus.clone());
    existing_by_id.set_chain_id("immediate");
    assert!(!existing_by_id.get_chain().is_compiled());

    let blank_el = LiteFlowChainELBuilder::create_chain(bus.clone());
    blank_el.set_chain_id("blank_el");
    assert!(matches!(
        blank_el.set_el(" \r\n "),
        Err(LiteflowError::Custom(message)) if message == "no el in this chain[blank_el]"
    ));

    let blank_uncompiled = Chain::new("blank_uncompiled", Vec::new());
    assert!(matches!(
        LiteFlowChainELBuilder::build_un_compile_chain(&bus, &blank_uncompiled),
        Err(LiteflowError::Custom(message))
            if message == "no el content in this unCompile chain[blank_uncompiled]"
    ));

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

    // retry 会把布尔 Node 包装成 RetryCondition；Java 按最终对象类型拒绝该 route。
    let invalid_wrapped_route = LiteFlowChainELBuilder::create_chain(bus.clone());
    invalid_wrapped_route.set_chain_id("invalid_wrapped_route");
    invalid_wrapped_route.set_el("THEN(body)").unwrap();
    invalid_wrapped_route.set_route("route_bool.retry(1)");
    assert!(matches!(
        invalid_wrapped_route.build(),
        Err(LiteflowError::RouteELInvalid(message))
            if message == "the route EL can only be a boolean node, or an AND or OR expression."
    ));

    // tag 属于 Node 自身属性，不产生包装 Condition，因此仍是合法 route。
    let tagged_node_route = LiteFlowChainELBuilder::create_chain(bus.clone());
    tagged_node_route.set_chain_id("tagged_node_route");
    tagged_node_route.set_el("THEN(body)").unwrap();
    tagged_node_route.set_route(r#"route_bool.tag("decision")"#);
    tagged_node_route
        .build()
        .expect("带 tag 的 Boolean Node 仍应保持 Node route 语义");

    // Java compileChain 总会 setRouteItem(this.route)。从已有 Chain 重建且没有
    // route EL 时，旧 routeItem 必须被 null 覆盖，不能残留上一次的决策条件。
    let mut stale_route_chain = Chain::new("stale_route", immediate.get_condition_list());
    stale_route_chain.set_el("THEN(body)");
    stale_route_chain.set_route_item(
        immediate
            .get_route_item()
            .expect("测试前置 Chain 应持有 route")
            .clone(),
    );
    let stale_route_builder = LiteFlowChainELBuilder::from_chain(bus.clone(), stale_route_chain);
    stale_route_builder
        .build()
        .expect("无 route EL 的已有 Chain 应可重新编译");
    let rebuilt_chains = bus.get_chain_map();
    assert!(
        rebuilt_chains["stale_route"].get_route_item().is_none(),
        "无 route EL 时必须清除旧 routeItem"
    );

    LiteflowConfigGetter::clean();
}

/// 注册覆盖所有 Operator 参数角色的节点，供构建期对象树测试使用。
fn register_builder_nodes(bus: &FlowBus) {
    for node_id in ["a", "b", "c", "body", "handler"] {
        bus.add_node(
            node_id,
            None,
            NodeTypeEnum::Common,
            Arc::new(cmp(|_| async { Ok(Value::Null) })),
        )
        .unwrap();
    }
    for node_id in ["p", "q", "stop"] {
        bus.add_node(
            node_id,
            None,
            NodeTypeEnum::Boolean,
            Arc::new(cmp(|_| async { Ok(Value::Bool(true)) })),
        )
        .unwrap();
    }
    bus.add_node(
        "selector",
        None,
        NodeTypeEnum::Switch,
        Arc::new(cmp(|_| async { Ok(Value::String("a".to_string())) })),
    )
    .unwrap();
    bus.add_node(
        "counter",
        None,
        NodeTypeEnum::For,
        Arc::new(cmp(|_| async { Ok(Value::from(1)) })),
    )
    .unwrap();
    bus.add_node(
        "items",
        None,
        NodeTypeEnum::Iterator,
        Arc::new(cmp(|_| async { Ok(Value::Array(vec![Value::from(1)])) })),
    )
    .unwrap();
}

/// 验证每个 Java EL Operator 都会生成真实 Condition，而不是只通过语法解析。
///
/// 对应 Java: `LiteFlowChainELBuilder#compile` 及各 `BaseELBuilder`。
#[test]
fn every_java_condition_shape_builds_a_real_executable_tree() {
    let bus = FlowBus::new();
    register_builder_nodes(&bus);
    let builder = LiteFlowChainELBuilder::create_chain(bus.clone());

    let valid_expressions = [
        "THEN(PRE(a), b, FINALLY(c))",
        "WHEN(a,b)",
        "IF(p,a).ELIF(q,b).ELSE(c)",
        "SWITCH(selector).TO(a,b).DEFAULT(c)",
        "FOR(counter).DO(body).BREAK(stop)",
        "FOR(2).DO(body).BREAK(stop)",
        "WHILE(p).DO(body).BREAK(stop)",
        "ITERATOR(items).DO(body).BREAK(stop)",
        "CATCH(body).DO(handler)",
        "AND(p,q)",
        "OR(p,q)",
        "NOT(p)",
        "PRE(a)",
        "FINALLY(a)",
        r#"THEN(a).bind("tenant","condition",true).retry(1).maxWaitMilliseconds(100)"#,
    ];

    for (index, source) in valid_expressions.into_iter().enumerate() {
        let expression =
            parse_el(source).unwrap_or_else(|error| panic!("{source} 应成功解析: {error}"));
        let chain = builder
            .build_chain(&format!("shape_{index}"), expression)
            .unwrap_or_else(|error| panic!("{source} 应构建真实 Executable: {error}"));
        assert_eq!(chain.get_condition_list().len(), 1, "{source}");
    }

    // route 的合法集合与 Java 最终对象类型检查一致：Node、AND、OR、NOT。
    for (index, route_source) in ["p", "AND(p,q)", "OR(p,q)", "NOT(p)"]
        .into_iter()
        .enumerate()
    {
        let chain = builder
            .build_route_chain(
                &format!("route_shape_{index}"),
                "orders",
                parse_el(route_source).unwrap(),
                parse_el("a").unwrap(),
            )
            .unwrap_or_else(|error| panic!("{route_source} 应为合法 route: {error}"));
        assert_eq!(chain.get_namespace(), "orders");
        assert!(chain.get_route_item().is_some());
    }

    // 子链只能出现在 Common 参数位置，作为布尔参数时必须保持 Java 参数错误。
    bus.add_chain("child", "THEN(a)").unwrap();
    assert!(matches!(
        builder.build_chain("chain_as_boolean", parse_el("IF(child,a)").unwrap()),
        Err(LiteflowError::Parse(message)) if message == "The parameter error."
    ));
}

/// 验证 validateWithEx 对所有嵌套结构按源码顺序报告第一个未注册对象。
///
/// 对应 Java: `LiteFlowChainELBuilder#validateWithEx` 与
/// `#buildDataNotFoundExceptionMsg`。
#[test]
fn validation_finds_the_first_unregistered_object_in_every_nested_shape() {
    let bus = FlowBus::new();
    register_builder_nodes(&bus);
    let builder = LiteFlowChainELBuilder::create_chain(bus);

    let invalid_expressions = [
        "THEN(a,missing)",
        "WHEN(a,missing)",
        "AND(p,missing)",
        "OR(p,missing)",
        "IF(p,a).ELIF(q,missing).ELSE(c)",
        "IF(p,a).ELSE(missing)",
        "SWITCH(selector).TO(a,missing).DEFAULT(c)",
        "SWITCH(selector).TO(a).DEFAULT(missing)",
        "FOR(counter).DO(body).BREAK(missing)",
        "FOR(2).DO(body).BREAK(missing)",
        "WHILE(p).DO(body).BREAK(missing)",
        "ITERATOR(items).DO(body).BREAK(missing)",
        "CATCH(body).DO(missing)",
        "NOT(missing)",
        "PRE(missing)",
        "FINALLY(missing)",
        "missing.retry(1).data(\"ignored\")",
    ];

    for source in invalid_expressions {
        let response = builder.validate_with_ex(source);
        assert!(!response.is_success(), "{source}");
        let cause = response
            .cause()
            .unwrap_or_else(|| panic!("{source} 应返回精确失败原因"))
            .to_string();
        assert!(
            cause.contains("[missing] is not exist"),
            "{source} 的首个缺失对象诊断错误: {cause}"
        );
    }
}

/// 验证声明式组件在构建边界保留方法和 PROCESS 生命周期校验。
///
/// 对应 Java: `ComponentInitializer#initComponent`。
#[test]
fn declarative_component_build_errors_are_not_replaced_by_fallback_nodes() {
    let bus = FlowBus::new();
    bus.register_decl("decl", Arc::new(NoProcessDeclComponent));
    bus.register_decl("decl_ok", Arc::new(ProcessDeclComponent));
    let builder = LiteFlowChainELBuilder::create_chain(bus);

    assert!(matches!(
        builder.build_chain("missing_method", parse_el("decl.unknown").unwrap()),
        Err(LiteflowError::NodeBuild(message))
            if message == "decl component[decl] method[unknown] not registered"
    ));
    assert!(matches!(
        builder.build_chain("missing_process", parse_el("decl").unwrap()),
        Err(LiteflowError::NodeBuild(message))
            if message == "decl component[decl] does not define the process method"
    ));
    assert_eq!(
        builder
            .build_chain("plain_process", parse_el("decl_ok").unwrap())
            .expect("无方法后缀的声明式组件应绑定 PROCESS 生命周期")
            .get_condition_list()
            .len(),
        1
    );
}

/// 验证 Condition 级 override bind 会递归清除每一种子树中的同名 Node bind。
///
/// 对应 Java: `BindOperator#clearNodeBindData`。
#[test]
fn condition_bind_override_traverses_every_java_condition_shape() {
    let bus = FlowBus::new();
    register_builder_nodes(&bus);
    let builder = LiteFlowChainELBuilder::create_chain(bus);
    let expressions = [
        r#"THEN(a.bind("tenant","node")).bind("tenant","condition",true)"#,
        r#"WHEN(a.bind("tenant","node"),b).bind("tenant","condition",true)"#,
        r#"IF(p.bind("tenant","node"),a.bind("tenant","node")).ELIF(q,b).ELSE(c).bind("tenant","condition",true)"#,
        r#"SWITCH(selector.bind("tenant","node")).TO(a.bind("tenant","node"),b).DEFAULT(c).bind("tenant","condition",true)"#,
        r#"FOR(counter.bind("tenant","node")).DO(body.bind("tenant","node")).BREAK(stop).bind("tenant","condition",true)"#,
        r#"FOR(2).DO(body.bind("tenant","node")).BREAK(stop).bind("tenant","condition",true)"#,
        r#"WHILE(p.bind("tenant","node")).DO(body.bind("tenant","node")).BREAK(stop).bind("tenant","condition",true)"#,
        r#"ITERATOR(items.bind("tenant","node")).DO(body.bind("tenant","node")).BREAK(stop).bind("tenant","condition",true)"#,
        r#"CATCH(body.bind("tenant","node")).DO(handler).bind("tenant","condition",true)"#,
        r#"NOT(p.bind("tenant","node")).bind("tenant","condition",true)"#,
        r#"PRE(a.bind("tenant","node")).bind("tenant","condition",true)"#,
        r#"FINALLY(a.bind("tenant","node")).bind("tenant","condition",true)"#,
        r#"a.bind("tenant","node").retry(1).bind("tenant","condition",true)"#,
    ];

    for (index, source) in expressions.into_iter().enumerate() {
        let expression =
            parse_el(source).unwrap_or_else(|error| panic!("{source} 应成功解析: {error}"));
        let chain = builder
            .build_chain(&format!("bind_shape_{index}"), expression)
            .unwrap_or_else(|error| panic!("{source} 应完成 bind override 构建: {error}"));
        assert_eq!(chain.get_condition_list().len(), 1, "{source}");
    }
}

/// 验证两阶段编译会在所有嵌套结构中识别并优先物化未编译子链。
///
/// 对应 Java: `LiteFlowChainELBuilder#compile` 放入 Chain 上下文后的依赖解析。
#[test]
fn two_phase_dependency_scan_traverses_every_java_condition_shape() {
    let bus = FlowBus::new();
    register_builder_nodes(&bus);

    let mut child = Chain::new("child", Vec::new());
    child.set_el("THEN(a)");
    child.set_compiled(false);
    bus.add_chain_phase1(child);

    let parent = LiteFlowChainELBuilder::create_chain(bus.clone());
    parent.set_chain_id("nested_parent");
    parent
        .set_el(
            "THEN(\
                WHEN(child),\
                IF(p,child).ELIF(q,child).ELSE(child),\
                SWITCH(selector).TO(child).DEFAULT(child),\
                FOR(counter).DO(child).BREAK(stop),\
                FOR(1).DO(child).BREAK(stop),\
                WHILE(p).DO(child).BREAK(stop),\
                ITERATOR(items).DO(child).BREAK(stop),\
                CATCH(child).DO(child),\
                PRE(child),\
                FINALLY(child),\
                child.retry(1)\
            )",
        )
        .unwrap();
    parent
        .build()
        .expect("父链构建应先物化所有嵌套位置引用的子链");

    let chains = bus.get_chain_map();
    assert!(chains["child"].is_compiled());
    assert!(chains["nested_parent"].is_compiled());
}

/// 验证 Rust 类型化 AST 扩展入口失败后可安全复用同一个 Builder。
///
/// Java 无此公共重载；其状态清理语义与 `compileChain` 的失败边界保持一致。
#[test]
fn parsed_ast_build_failure_does_not_poison_the_next_build() {
    let bus = FlowBus::new();
    register_builder_nodes(&bus);
    let builder = LiteFlowChainELBuilder::create_chain(bus.clone());

    assert!(matches!(
        builder.build_parsed_chain("invalid_ast", parse_el("IF(a,b)").unwrap()),
        Err(LiteflowError::Parse(message))
            if message == "The node[a] must be boolean type Node."
    ));
    builder
        .build_parsed_chain("valid_after_failure", parse_el("a").unwrap())
        .expect("失败后的同一 Builder 应能构建并注册下一棵 AST");
    let chains = bus.get_chain_map();
    assert!(!chains.contains_key("invalid_ast"));
    assert!(chains["valid_after_failure"].is_compiled());

    // Boolean 字面量不是注册表对象，validateWithEx 不应把 true/false 误报为节点。
    assert!(builder.validate("WHILE(false).DO(a)"));
}
