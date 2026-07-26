//! S3 builder/prop 与 LiteFlowNodeBuilder 验收测试。
//!
//! 覆盖 serde 配置映射、普通组件动态注册、脚本文本/脚本文件构建、
//! Java v2.16 NodeType code 以及构建前校验。

use liteflow_core::{
    ChainPropBean, ConditionTypeEnum, FlowBus, LiteFlowNodeBuilder, LiteflowError, NodePropBean,
    NodeTypeEnum, cmp,
};
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn prop_beans_use_serde_for_java_json_shape() {
    let node: NodePropBean = serde_json::from_str(
        r#"{
            "id":"calculate",
            "name":"计算",
            "class":"demo.CalculateCmp",
            "value":"40 + 2",
            "type":"script",
            "language":"rhai"
        }"#,
    )
    .unwrap();
    assert_eq!(node.id(), Some("calculate"));
    assert_eq!(node.clazz(), Some("demo.CalculateCmp"));
    assert_eq!(node.script(), Some("40 + 2"));
    assert_eq!(node.node_type(), Some("script"));

    let chain = ChainPropBean::default()
        .set_cond_value_str("a,b")
        .set_group("g1")
        .set_error_resume("false")
        .set_any("true")
        .set_thread_executor_class("FastExecutor")
        .set_condition_type(ConditionTypeEnum::When);
    let json = serde_json::to_value(chain).unwrap();
    assert_eq!(json["condValueStr"], "a,b");
    assert_eq!(json["threadExecutorClass"], "FastExecutor");
    assert_eq!(json["conditionType"], "when");
}

#[tokio::test]
async fn common_node_builder_registers_component_and_name() {
    let bus = FlowBus::new();
    LiteFlowNodeBuilder::create_common_node(&bus)
        .set_id("hello")
        .set_name("问候组件")
        .set_component(cmp(|_| async { Ok(Value::Null) }))
        .build()
        .unwrap();
    bus.add_chain("builder_common", "THEN(hello)").unwrap();

    let response = bus.execute("builder_common").await;
    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(response.steps[0].node_id, "hello");
    assert_eq!(response.steps[0].node_name, "问候组件");
}

#[tokio::test]
async fn script_node_builder_and_node_prop_execute_real_flow() {
    let bus = FlowBus::new();
    let prop: NodePropBean = serde_json::from_str(
        r#"{"id":"allowed","name":"许可判断","type":"boolean_script","value":"true"}"#,
    )
    .unwrap();
    LiteFlowNodeBuilder::from_prop(&bus, prop)
        .unwrap()
        .build()
        .unwrap();

    let body_count = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&body_count);
    bus.register(
        "body",
        cmp(move |_| {
            let counter = Arc::clone(&counter);
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(Value::Null)
            }
        }),
    );
    bus.add_chain("builder_script", "IF(allowed,body)").unwrap();

    let response = bus.execute("builder_script").await;
    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(body_count.load(Ordering::SeqCst), 1);
    assert_eq!(response.steps[0].node_name, "许可判断");
}

#[tokio::test]
async fn script_file_builder_reads_through_path_content_parser() {
    let bus = FlowBus::new();
    let file = std::env::temp_dir().join(format!(
        "liteflow-rust-node-builder-{}-{}.rhai",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    std::fs::write(&file, "2").unwrap();

    LiteFlowNodeBuilder::create_script_for_node(&bus)
        .set_id("loop_count")
        .set_file(file.to_string_lossy())
        .build()
        .unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&count);
    bus.register(
        "body",
        cmp(move |_| {
            let counter = Arc::clone(&counter);
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(Value::Null)
            }
        }),
    );
    bus.add_chain("builder_file", "FOR(loop_count).DO(body)")
        .unwrap();

    let response = bus.execute("builder_file").await;
    let _ = std::fs::remove_file(file);
    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn json_yml_and_xml_rules_share_node_prop_builder_path() {
    let cases: [(
        &str,
        fn(&FlowBus, &str) -> liteflow_core::LFResult<Vec<String>>,
    ); 3] = [
        (
            r#"{"flow":{"nodes":{"node":[{"id":"script_node","type":"script","value":"40 + 2"}]},"chain":[{"id":"configured","body":"THEN(script_node)"}]}}"#,
            liteflow_core::rule::load_json_str,
        ),
        (
            "flow:\n  nodes:\n    node:\n      - id: script_node\n        type: script\n        value: '40 + 2'\n  chain:\n    - id: configured\n      body: THEN(script_node)\n",
            liteflow_core::rule::load_yml_str,
        ),
        (
            r#"<flow><nodes><node id="script_node" type="script">40 + 2</node></nodes><chain id="configured"><body>THEN(script_node)</body></chain></flow>"#,
            liteflow_core::rule::load_xml_str,
        ),
    ];

    for (source, load) in cases {
        let bus = FlowBus::new();
        assert_eq!(load(&bus, source).unwrap(), vec!["configured"]);
        let response = bus.execute("configured").await;
        assert!(response.is_success(), "{:?}", response.cause);
    }
}

#[test]
fn node_builder_validates_id_type_and_supported_codes() {
    assert!(matches!(
        LiteFlowNodeBuilder::create_node(&FlowBus::new()).build(),
        Err(LiteflowError::NodeBuild(message)) if message.contains("id is blank")
    ));
    assert!(matches!(
        LiteFlowNodeBuilder::create_node(&FlowBus::new())
            .set_id("a")
            .build(),
        Err(LiteflowError::NodeBuild(message)) if message.contains("type is null")
    ));
    assert_eq!(
        NodeTypeEnum::get_enum_by_code("boolean"),
        Some(NodeTypeEnum::Boolean)
    );
    assert_eq!(
        NodeTypeEnum::get_enum_by_code("boolean_script"),
        Some(NodeTypeEnum::BooleanScript)
    );
    assert_eq!(
        NodeTypeEnum::get_enum_by_code("fallback"),
        Some(NodeTypeEnum::Fallback)
    );
}
