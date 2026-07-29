//! Parser 对象链语义测试：基类批量解析、Provider 选择与自定义内容源。

use std::collections::HashSet;
use std::sync::Arc;

use liteflow_core::parser::{
    BaseJsonFlowParser, BaseXmlFlowParser, FlowParserProvider, NodeConvertHelper, ParserHelper,
    RuleDefinitionPlan,
};
use liteflow_core::util::RuleParsePluginUtil;
use liteflow_core::{
    FlowBus, FlowParserTypeEnum, LiteflowConfig, LiteflowConfigGetter, NodePropBean, NodeTypeEnum,
    ParseModeEnum, cmp,
};
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

/// 验证延迟规则计划进入物化阶段后，不会再次受进程级 parseMode 影响。
///
/// 对应 Java `PARSE_ONE_ON_FIRST_EXEC` 首次执行时
/// `LiteFlowChainELBuilder#buildUnCompileChain` 的立即编译语义。
#[tokio::test]
async fn delayed_rule_plan_materialization_ignores_global_parse_mode() {
    let bus = FlowBus::new();
    bus.register(
        "materializedNode",
        cmp(|context| async move {
            context.set_data("materialized", json!(true));
            Ok(Value::Null)
        }),
    );
    let plan = BaseJsonFlowParser::new(bus.clone())
        .collect(&[r#"{
            "flow": {
                "chain": [{
                    "id": "materializedChain",
                    "body": "THEN(materializedNode)"
                }]
            }
        }"#
        .to_string()])
        .unwrap();

    let mut global_config = LiteflowConfig::default();
    global_config.set_parse_mode(ParseModeEnum::ParseOneOnFirstExec);
    LiteflowConfigGetter::set_liteflow_config(global_config);
    let build_result = plan.build_chain(&bus, "materializedChain");
    LiteflowConfigGetter::clean();
    build_result.unwrap();

    let response = bus.execute("materializedChain").await;
    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(response.data("materialized"), Some(json!(true)));
    assert_eq!(response.steps.len(), 1, "物化后的链不能是未编译空占位链");
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

/// 验证 ParserHelper 保留 Java JSON 解析的缺省、兼容与失败边界。
///
/// 对应 Java: `ParserHelper#parseNodeJson`、`#parseChainJson` 与
/// `#parseOneChain(JsonNode)`。
#[test]
fn parser_helper_json_boundaries_preserve_java_contracts() {
    let mut plan = RuleDefinitionPlan::new();
    assert!(matches!(
        ParserHelper::parse_node_json(&[json!({"notFlow": {}})], &mut plan),
        Err(liteflow_core::LiteflowError::Rule(message)) if message == "missing flow"
    ));

    ParserHelper::parse_node_json(&[json!({"flow": {"nodes": {"notNode": []}}})], &mut plan)
        .expect("没有 node 数组时 Java 语义是跳过");
    assert_eq!(plan.chain_count(), 0);

    let invalid_node = ParserHelper::parse_node_json(
        &[json!({"flow": {"nodes": {"node": [
            {"id": "broken", "type": {"invalid": true}}
        ]}}})],
        &mut plan,
    )
    .unwrap_err();
    assert!(matches!(
        invalid_node,
        liteflow_core::LiteflowError::Rule(message)
            if message.contains("invalid node[broken] property")
    ));

    let mut chain_ids = HashSet::new();
    ParserHelper::parse_chain_json(
        &[json!({"flow": {}}), json!({"notFlow": {}})],
        &mut chain_ids,
        &mut plan,
    )
    .expect("缺少 chain 数组的文档应被跳过");
    assert!(chain_ids.is_empty());

    assert!(matches!(
        ParserHelper::parse_one_chain(&json!({"body": "THEN(a)"})),
        Err(liteflow_core::LiteflowError::Rule(message))
            if message == "chain missing id/name"
    ));
    assert!(matches!(
        ParserHelper::parse_one_chain(&json!({"id": "routeOnly", "route": "p"})),
        Err(liteflow_core::LiteflowError::Rule(message))
            if message == "If you have defined the field route, then you must define the field body in chain[routeOnly]"
    ));
    assert!(matches!(
        ParserHelper::parse_one_chain(&json!({"id": "missingCondition"})),
        Err(liteflow_core::LiteflowError::Rule(message))
            if message == "chain[missingCondition] missing condition"
    ));
    assert!(matches!(
        ParserHelper::parse_one_chain(&json!({
            "id": "missingValue",
            "condition": [{"type": "then"}]
        })),
        Err(liteflow_core::LiteflowError::Rule(message))
            if message == "chain[missingValue] condition missing value"
    ));

    let legacy = ParserHelper::parse_one_chain(&json!({
        "name": "legacy",
        "namespace": "",
        "extends": "abstractParent",
        "threadPoolExecutorClass": "legacy-pool",
        "condition": [
            {"type": "then", "value": "a"},
            {"type": "when", "value": "b"}
        ]
    }))
    .unwrap()
    .expect("Java 旧 condition 数组应转换为 EL");
    assert_eq!(legacy.id, "legacy");
    assert_eq!(legacy.namespace, "default");
    assert_eq!(legacy.extends.as_deref(), Some("abstractParent"));
    assert_eq!(
        legacy.thread_pool_executor_class.as_deref(),
        Some("legacy-pool")
    );
    assert_eq!(legacy.body, "THEN(THEN(a),WHEN(b))");

    let single = ParserHelper::parse_one_chain(&json!({
        "id": "single",
        "condition": [{"value": "a"}]
    }))
    .unwrap()
    .expect("单个旧 condition 应直接生成对应 EL");
    assert_eq!(single.body, "THEN(a)");
}

/// 验证 XML 流式读取器在未知元素、空链、禁用项和截断输入上的精确行为。
///
/// 对应 Java: `ParserHelper#parseNodeDocument` 与 `#parseChainDocument`。
#[test]
fn parser_helper_xml_boundaries_preserve_java_contracts() {
    let mut plan = RuleDefinitionPlan::new();
    ParserHelper::parse_node_document(
        &[r#"
            <flow>
              <metadata><nested/></metadata>
              <nodes>
                <unknown><node id="ignored" type="script">()</node></unknown>
                <node id="emptyScript" type="script" value="()"/>
                <node id="disabledScript" type="script" enable="false">()</node>
              </nodes>
            </flow>
        "#
        .to_string()],
        &mut plan,
    )
    .expect("未知 XML 元素应完整跳过，Empty node 应保留属性");

    assert!(matches!(
        ParserHelper::parse_node_document(
            &["<flow><nodes><node id=\"broken\" type=\"script\">".to_string()],
            &mut RuleDefinitionPlan::new(),
        ),
        Err(liteflow_core::LiteflowError::Rule(message))
            if message.contains("unclosed <node>")
    ));

    let disabled_only =
        vec![r#"<flow><chain id="disabled" enable="false">THEN(a)</chain></flow>"#.to_string()];
    let mut disabled_plan = RuleDefinitionPlan::new();
    ParserHelper::parse_chain_document(&disabled_only, &mut HashSet::new(), &mut disabled_plan)
        .expect("禁用 XML Chain 应被跳过");
    assert_eq!(disabled_plan.chain_count(), 0);

    assert!(matches!(
        ParserHelper::parse_chain_document(
            &["<flow><chain>THEN(a)</chain></flow>".to_string()],
            &mut HashSet::new(),
            &mut RuleDefinitionPlan::new(),
        ),
        Err(liteflow_core::LiteflowError::Rule(message))
            if message == "missing chain id in expression"
    ));
    assert!(matches!(
        ParserHelper::parse_chain_document(
            &["<flow><chain id=\"empty\"/></flow>".to_string()],
            &mut HashSet::new(),
            &mut RuleDefinitionPlan::new(),
        ),
        Err(liteflow_core::LiteflowError::Rule(message))
            if message == "chain[empty] has empty EL"
    ));
    assert!(matches!(
        ParserHelper::parse_chain_document(
            &["<flow><chain id=\"routeOnly\"><route>p</route></chain></flow>".to_string()],
            &mut HashSet::new(),
            &mut RuleDefinitionPlan::new(),
        ),
        Err(liteflow_core::LiteflowError::Rule(message))
            if message == "If you have defined the tag <route>, then you must define the tag <body> in chain[routeOnly]"
    ));
    assert!(matches!(
        ParserHelper::parse_chain_document(
            &["<flow><chain id=\"unclosed\"><body>THEN(a)</body>".to_string()],
            &mut HashSet::new(),
            &mut RuleDefinitionPlan::new(),
        ),
        Err(liteflow_core::LiteflowError::Rule(message))
            if message == "chain[unclosed] unclosed"
    ));
    assert!(matches!(
        ParserHelper::parse_chain_document(
            &["<flow><chain id=\"same\">THEN(a)</chain><chain id=\"same\">THEN(a)</chain></flow>".to_string()],
            &mut HashSet::new(),
            &mut RuleDefinitionPlan::new(),
        ),
        Err(liteflow_core::LiteflowError::ChainDuplicate(message))
            if message == "[chain name duplicate] chainName=same"
    ));
}

/// 验证延迟规则计划会遍历所有 EL 结构，只物化目标 Chain 的完整依赖闭包。
///
/// 对应 Java: `ParserHelper#parseChainJson` 的先登记、后编译与继承处理阶段。
#[test]
fn rule_definition_plan_materializes_all_nested_reference_shapes_and_errors() {
    let bus = FlowBus::new();
    for node_id in ["a", "body", "handler"] {
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
        Arc::new(cmp(|_| async { Ok(Value::String("child".to_string())) })),
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
        Arc::new(cmp(|_| async { Ok(json!([1])) })),
    )
    .unwrap();

    let documents = vec![json!({
        "flow": {
            "chain": [
                {"id": "child", "body": "THEN(a)"},
                {
                    "id": "nested",
                    "route": "AND(p,q)",
                    "body": "THEN(\
                        WHEN(child),\
                        IF(p,child).ELIF(q,child).ELSE(child),\
                        SWITCH(selector).TO(child).DEFAULT(child),\
                        FOR(counter).DO(child).BREAK(stop),\
                        FOR(1).DO(child).BREAK(stop),\
                        WHILE(p).DO(child).BREAK(stop),\
                        ITERATOR(items).DO(child).BREAK(stop),\
                        CATCH(child).DO(handler),\
                        NOT(p),\
                        PRE(child),\
                        FINALLY(child),\
                        child.retry(1)\
                    )"
                }
            ]
        }
    })];
    let mut plan = RuleDefinitionPlan::new();
    ParserHelper::parse_chain_json(&documents, &mut HashSet::new(), &mut plan).unwrap();
    assert_eq!(plan.chain_count(), 2);
    plan.build_chain(&bus, "nested")
        .expect("目标 Chain 的全部嵌套依赖应递归物化");
    let chains = bus.get_chain_map();
    assert!(chains["child"].is_compiled());
    assert!(chains["nested"].is_compiled());
    plan.build_chain(&bus, "nested")
        .expect("已经物化的 Chain 应直接返回");

    let empty_plan = RuleDefinitionPlan::new();
    assert!(matches!(
        empty_plan.build_chain(&FlowBus::new(), "missing"),
        Err(liteflow_core::LiteflowError::ChainNotFound(message))
            if message == "[chain not found] chainId=missing"
    ));

    let mut abstract_plan = RuleDefinitionPlan::new();
    ParserHelper::parse_chain_json(
        &[json!({"flow": {"chain": [
            {"id": "abstractParent", "body": "THEN(a,{{next}})"}
        ]}})],
        &mut HashSet::new(),
        &mut abstract_plan,
    )
    .unwrap();
    assert!(matches!(
        abstract_plan.build_chain(&FlowBus::new(), "abstractParent"),
        Err(liteflow_core::LiteflowError::ChainNotFound(message))
            if message == "[abstract chain cannot execute] chainId=abstractParent"
    ));

    let mut reference_cycle = RuleDefinitionPlan::new();
    ParserHelper::parse_chain_json(
        &[json!({"flow": {"chain": [
            {"id": "cycleA", "body": "THEN(cycleB)"},
            {"id": "cycleB", "body": "THEN(cycleA)"}
        ]}})],
        &mut HashSet::new(),
        &mut reference_cycle,
    )
    .unwrap();
    assert!(matches!(
        reference_cycle.build_chain(&FlowBus::new(), "cycleA"),
        Err(liteflow_core::LiteflowError::Parse(message))
            if message == "cyclic chain reference detected: cycleA"
    ));

    let mut inheritance_cycle = RuleDefinitionPlan::new();
    ParserHelper::parse_chain_json(
        &[json!({"flow": {"chain": [
            {"id": "inheritA", "extends": "inheritB", "body": "{{next}} = a;"},
            {"id": "inheritB", "extends": "inheritA", "body": "{{next}} = a;"}
        ]}})],
        &mut HashSet::new(),
        &mut inheritance_cycle,
    )
    .unwrap();
    assert!(matches!(
        inheritance_cycle.build_chain(&FlowBus::new(), "inheritA"),
        Err(liteflow_core::LiteflowError::Parse(message))
            if message == "cyclic chain inheritance detected: inheritA"
    ));
}
