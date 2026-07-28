//! Parser 对象链语义测试：基类批量解析、Provider 选择与自定义内容源。

use std::collections::HashSet;
use std::sync::Arc;

use liteflow_core::parser::{
    BaseJsonFlowParser, BaseXmlFlowParser, FlowParserProvider, NodeConvertHelper, ParserHelper,
    RuleDefinitionPlan,
};
use liteflow_core::util::RuleParsePluginUtil;
use liteflow_core::{FlowBus, FlowParserTypeEnum, NodePropBean, cmp};
use serde_json::{Value, json};

#[tokio::test]
async fn base_json_parser_resolves_inheritance_across_content_list() {
    let bus = FlowBus::new();
    for id in ["a", "b"] {
        bus.register(id, cmp(|_| async { Ok(Value::Null) }));
    }

    let parent = r#"{"flow":{"chain":[{"id":"parent","body":"THEN(a, {{next}})"}]}}"#.to_string();
    let child = r#"{"flow":{"chain":[{"id":"child","extends":"parent","body":"{{next}} = b;"}]}}"#
        .to_string();

    let ids = BaseJsonFlowParser::new(bus.clone())
        .parse(&[parent, child])
        .unwrap();

    assert_eq!(ids, vec!["child"]);
    assert!(!bus.contains_chain("parent"));
    assert!(bus.execute("child").await.is_success());
}

#[test]
fn provider_selects_real_local_parser_objects_by_suffix() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    let provider = FlowParserProvider::new(bus);

    let json = provider.lookup("rules.el.json").unwrap();
    let xml = provider.lookup("rules.xml").unwrap();
    let yml = provider.lookup("rules.yml").unwrap();

    assert_eq!(
        json.parse(&[r#"{"flow":{"chain":[{"id":"jsonChain","body":"THEN(a)"}]}}"#.to_string()])
            .unwrap(),
        vec!["jsonChain"]
    );
    assert_eq!(
        xml.parse(&["<flow><chain id=\"xmlChain\">THEN(a)</chain></flow>".to_string()])
            .unwrap(),
        vec!["xmlChain"]
    );
    assert_eq!(
        yml.parse(&["flow:\n  chain:\n    - id: ymlChain\n      body: THEN(a)\n".to_string()])
            .unwrap(),
        vec!["ymlChain"]
    );
}

#[tokio::test]
async fn provider_executes_registered_class_json_parser() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    let provider = FlowParserProvider::new(bus.clone());
    provider.register_class_parser(
        "example.CustomJsonParser",
        FlowParserTypeEnum::TypeElJson,
        Arc::new(|| {
            Ok(r#"{"flow":{"chain":[{"id":"customChain","body":"THEN(a)"}]}}"#.to_string())
        }),
    );

    let parser = provider.lookup("el_json:example.CustomJsonParser").unwrap();
    let ids = parser.parse_main(&[]).unwrap();

    assert_eq!(ids, vec!["customChain"]);
    assert!(bus.execute("customChain").await.is_success());
}

#[test]
fn provider_rejects_unregistered_or_mismatched_class_parser() {
    let provider = FlowParserProvider::new(FlowBus::new());
    provider.register_class_parser(
        "example.CustomJsonParser",
        FlowParserTypeEnum::TypeElJson,
        Arc::new(|| Ok(r#"{"flow":{"chain":[]}}"#.to_string())),
    );

    assert!(provider.lookup("el_xml:example.CustomJsonParser").is_err());
    assert!(provider.lookup("example.MissingParser").is_err());
    assert!(provider.lookup("unsupported:example.Parser").is_err());
}

#[test]
fn node_convert_helper_matches_java_colon_key_semantics() {
    let node = NodeConvertHelper::convert("scriptNode:script:demo:rhai:false").unwrap();
    assert_eq!(node.node_id(), "scriptNode");
    assert_eq!(node.node_type(), "script");
    assert_eq!(node.name(), "demo");
    assert_eq!(node.language(), Some("rhai"));
    assert!(!node.enable());

    assert!(NodeConvertHelper::convert("scriptNode").is_none());
    assert!(NodeConvertHelper::convert("scriptNode::script").is_none());

    // Java 正则会保留第一组完整相邻段，并忽略双冒号后的孤立段。
    let partial = NodeConvertHelper::convert("scriptNode:script::ignored").unwrap();
    assert_eq!(partial.node_id(), "scriptNode");
    assert_eq!(partial.node_type(), "script");
    assert_eq!(partial.name(), "");
}

#[tokio::test]
async fn rule_parse_plugin_xml_round_trips_through_real_xml_parser() {
    let bus = FlowBus::new();
    let mut node = NodeConvertHelper::convert("scriptNode:script:demo:rhai:true").unwrap();
    node.set_script("()");

    let node_xml = RuleParsePluginUtil::to_script_xml(&node);
    let chain = RuleParsePluginUtil::parse_chain_key("pluginChain:true");
    let chain_xml = chain.to_el_xml("THEN(scriptNode)");
    let document = format!("<flow><nodes>{node_xml}</nodes>{chain_xml}</flow>");

    let ids = BaseXmlFlowParser::new(bus.clone())
        .parse(&[document])
        .unwrap();

    assert_eq!(ids, vec!["pluginChain"]);
    assert!(bus.contains_node("scriptNode"));
    assert!(bus.execute("pluginChain").await.is_success());
}

#[tokio::test]
async fn xml_rule_plan_replaces_existing_script_nodes_and_chain_on_refresh() {
    let bus = FlowBus::new();
    let original = r#"
        <flow>
          <nodes>
            <node id="refreshScript" type="script" language="rhai"><![CDATA[
              data["version"] = 1;
            ]]></node>
          </nodes>
          <chain id="refreshChain">THEN(refreshScript)</chain>
        </flow>
    "#;
    BaseXmlFlowParser::new(bus.clone())
        .parse(&[original.to_string()])
        .unwrap();
    assert_eq!(
        bus.execute("refreshChain").await.data("version"),
        Some(json!(1))
    );

    let updated = r#"
        <flow>
          <nodes>
            <node id="refreshScript" type="script" language="rhai"><![CDATA[
              data["version"] = 2;
            ]]></node>
            <node id="addedScript" type="script" language="rhai"><![CDATA[
              data["added"] = true;
            ]]></node>
          </nodes>
          <chain id="refreshChain">THEN(refreshScript, addedScript)</chain>
        </flow>
    "#;
    BaseXmlFlowParser::new(bus.clone())
        .parse(&[updated.to_string()])
        .unwrap();

    let response = bus.execute("refreshChain").await;
    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(response.data("version"), Some(json!(2)));
    assert_eq!(response.data("added"), Some(json!(true)));
}

#[test]
fn rule_parse_plugin_key_flags_match_java_defaults() {
    let disabled = RuleParsePluginUtil::parse_chain_key("chainA:false");
    assert_eq!(disabled.get_id(), "chainA");
    assert!(disabled.is_disable());
    assert_eq!(disabled.get_enable(), "false");

    let malformed = RuleParsePluginUtil::parse_chain_key("chain:A:true");
    assert_eq!(malformed.get_id(), "chain:A:true");
    assert_eq!(malformed.get_enable(), "true");
    assert!(malformed.is_enable());

    assert_eq!(
        RuleParsePluginUtil::parse_id_key("chainA:FALSE"),
        (false, "chainA".to_string())
    );
    assert_eq!(
        RuleParsePluginUtil::parse_id_key("chain:A:true"),
        (true, "chain:A:true".to_string())
    );
}

#[test]
fn parser_helper_reports_java_node_type_errors() {
    let missing_type = ParserHelper::build_node(
        &FlowBus::new(),
        NodePropBean::default()
            .set_id("missingType")
            .set_script("()"),
    )
    .unwrap_err();
    assert!(matches!(
        missing_type,
        liteflow_core::LiteflowError::NodeTypeCanNotGuess(_)
    ));

    let unsupported_type = ParserHelper::build_node(
        &FlowBus::new(),
        NodePropBean::default()
            .set_id("badType")
            .set_type("unknown"),
    )
    .unwrap_err();
    assert!(matches!(
        unsupported_type,
        liteflow_core::LiteflowError::NodeTypeNotSupport(_)
    ));

    let missing_class = ParserHelper::build_node(
        &FlowBus::new(),
        NodePropBean::default()
            .set_id("classNode")
            .set_clazz("example.MissingNode"),
    )
    .unwrap_err();
    assert!(matches!(
        missing_class,
        liteflow_core::LiteflowError::NodeClassNotFound(_)
    ));
}

#[tokio::test]
async fn parser_helper_java_named_json_entries_drive_real_build_plan() {
    let bus = FlowBus::new();
    let documents = vec![json!({
        "flow": {
            "nodes": {
                "node": [
                    {
                        "id": "jsonScript",
                        "type": "script",
                        "language": "rhai",
                        "value": "()"
                    },
                    {
                        "id": "disabledScript",
                        "type": "script",
                        "language": "rhai",
                        "value": "()",
                        "enable": false
                    }
                ]
            },
            "chain": [
                {
                    "id": "jsonHelperChain",
                    "body": "THEN(jsonScript)",
                    "threadPoolExecutorClass": "json-pool"
                },
                {
                    "id": "disabledChain",
                    "body": "THEN(disabledScript)",
                    "enable": "false"
                }
            ]
        }
    })];
    let mut plan = RuleDefinitionPlan::new();
    ParserHelper::parse_node_json(&documents, &mut plan).unwrap();
    ParserHelper::parse_chain_json(&documents, &mut HashSet::new(), &mut plan).unwrap();

    assert_eq!(plan.build_all(&bus).unwrap(), vec!["jsonHelperChain"]);
    assert!(bus.contains_node("jsonScript"));
    assert!(!bus.contains_node("disabledScript"));
    assert_eq!(
        bus.get_chain_map()["jsonHelperChain"].get_thread_pool_executor_class(),
        Some("json-pool")
    );
    assert!(bus.execute("jsonHelperChain").await.is_success());

    // Java parseOneChain 对禁用链返回 null；Rust 以 Option 显式表达。
    assert!(
        ParserHelper::parse_one_chain(&json!({
            "id": "disabled",
            "body": "THEN(jsonScript)",
            "enable": false
        }))
        .unwrap()
        .is_none()
    );

    let duplicate_documents = vec![json!({
        "flow": {
            "chain": [
                {"id": "same", "body": "THEN(jsonScript)"},
                {"id": "same", "body": "THEN(jsonScript)"}
            ]
        }
    })];
    let duplicate = ParserHelper::parse_chain_json(
        &duplicate_documents,
        &mut HashSet::new(),
        &mut RuleDefinitionPlan::new(),
    )
    .unwrap_err();
    assert!(matches!(
        duplicate,
        liteflow_core::LiteflowError::ChainDuplicate(_)
    ));
}

#[tokio::test]
async fn parser_helper_java_named_xml_entries_drive_real_build_plan() {
    let bus = FlowBus::new();
    let documents = vec![
        r#"
        <flow>
          <nodes>
            <node id="xmlScript" type="script" language="rhai"><![CDATA[()]]></node>
            <node id="disabledXmlScript" type="script" language="rhai" enable="false"><![CDATA[()]]></node>
          </nodes>
          <chain id="xmlHelperChain" threadPoolExecutorClass="xml-pool">THEN(xmlScript)</chain>
          <chain id="disabledXmlChain" enable="false">THEN(disabledXmlScript)</chain>
        </flow>
        "#
        .to_string(),
    ];
    let mut plan = RuleDefinitionPlan::new();
    ParserHelper::parse_node_document(&documents, &mut plan).unwrap();
    ParserHelper::parse_chain_document(&documents, &mut HashSet::new(), &mut plan).unwrap();

    assert_eq!(plan.build_all(&bus).unwrap(), vec!["xmlHelperChain"]);
    assert!(bus.contains_node("xmlScript"));
    assert!(!bus.contains_node("disabledXmlScript"));
    assert_eq!(
        bus.get_chain_map()["xmlHelperChain"].get_thread_pool_executor_class(),
        Some("xml-pool")
    );
    assert!(bus.execute("xmlHelperChain").await.is_success());
}
