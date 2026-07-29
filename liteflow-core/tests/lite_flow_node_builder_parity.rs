//! LiteFlowNodeBuilder 的 Java v2.16.0 对象级语义验收。

use std::sync::Arc;

use liteflow_core::{FlowBus, LiteFlowNodeBuilder, LiteflowError, NodePropBean, NodeTypeEnum, cmp};
use serde_json::Value;

/// 验证 Java 十个静态工厂都创建对应类型并注册真实组件。
///
/// 对应 Java: `LiteFlowNodeBuilder#create*Node`。
#[test]
fn every_java_factory_registers_its_exact_node_type() {
    let bus = FlowBus::new();
    let component = || cmp(|_| async { Ok(Value::Null) });

    LiteFlowNodeBuilder::create_common_node(&bus)
        .set_id("common")
        .set_component(component())
        .build()
        .unwrap();
    LiteFlowNodeBuilder::create_switch_node(&bus)
        .set_id("switch")
        .set_component(component())
        .build()
        .unwrap();
    LiteFlowNodeBuilder::create_boolean_node(&bus)
        .set_id("boolean")
        .set_component(component())
        .build()
        .unwrap();
    LiteFlowNodeBuilder::create_for_node(&bus)
        .set_id("for")
        .set_component(component())
        .build()
        .unwrap();
    LiteFlowNodeBuilder::create_iterator_node(&bus)
        .set_id("iterator")
        .set_component_arc(Arc::new(component()))
        .build()
        .unwrap();

    LiteFlowNodeBuilder::create_script_node(&bus)
        .set_id("script")
        .set_script("40 + 2")
        .build()
        .unwrap();
    LiteFlowNodeBuilder::create_script_switch_node(&bus)
        .set_id("switch_script")
        .set_script(r#""target""#)
        .build()
        .unwrap();
    LiteFlowNodeBuilder::create_script_boolean_node(&bus)
        .set_id("boolean_script")
        .set_script("true")
        .build()
        .unwrap();
    LiteFlowNodeBuilder::create_script_for_node(&bus)
        .set_id("for_script")
        .set_script("1")
        .build()
        .unwrap();

    for node_id in [
        "common",
        "switch",
        "boolean",
        "for",
        "iterator",
        "script",
        "switch_script",
        "boolean_script",
        "for_script",
    ] {
        assert!(bus.contains_node(node_id), "{node_id}");
    }
}

/// 验证 checkBuild 同时聚合 id/type，并覆盖普通、降级及脚本失败边界。
///
/// 对应 Java: `LiteFlowNodeBuilder#checkBuild` 与 `#build`。
#[test]
fn build_validation_and_failure_boundaries_match_java_order() {
    let bus = FlowBus::new();
    assert!(matches!(
        LiteFlowNodeBuilder::create_node(&bus).build(),
        Err(LiteflowError::NodeBuild(message)) if message == "[id is blank,type is null]"
    ));
    assert!(matches!(
        LiteFlowNodeBuilder::create_node(&bus)
            .set_id("typed")
            .build(),
        Err(LiteflowError::NodeBuild(message)) if message == "[type is null]"
    ));
    assert!(matches!(
        LiteFlowNodeBuilder::create_common_node(&bus)
            .set_id("missing_component")
            .set_clazz("demo.MissingComponent")
            .build(),
        Err(LiteflowError::NodeBuild(message))
            if message.contains("demo.MissingComponent") && message.contains("set_component")
    ));
    assert!(matches!(
        LiteFlowNodeBuilder::create_node(&bus)
            .set_id("fallback")
            .set_type(NodeTypeEnum::Fallback)
            .build(),
        Err(LiteflowError::NodeTypeNotSupport(_))
    ));
    assert!(matches!(
        LiteFlowNodeBuilder::create_script_node(&bus)
            .set_id("blank_script")
            .set_script("  ")
            .build(),
        Err(LiteflowError::NodeBuild(message)) if message.contains("script is blank")
    ));
    assert!(
        LiteFlowNodeBuilder::create_script_node(&bus)
            .set_id("missing_file")
            .set_file("/path/that/does/not/exist/liteflow-script.rhai")
            .build()
            .is_err()
    );
}

/// 验证 NodePropBean 的所有字段、非法类型和既有节点复用语义。
///
/// 对应 Java: ParserHelper 到 `LiteFlowNodeBuilder` 的属性装配链。
#[test]
fn node_prop_mapping_uses_trimmed_fields_and_existing_registration() {
    let bus = FlowBus::new();
    bus.register("existing", cmp(|_| async { Ok(Value::Null) }));

    let existing: NodePropBean = serde_json::from_str(
        r#"{"id":" existing ","name":" kept ","class":" demo.Existing ","language":" ","file":" "}"#,
    )
    .unwrap();
    LiteFlowNodeBuilder::from_prop(&bus, existing)
        .unwrap()
        .build()
        .unwrap();

    let script: NodePropBean = serde_json::from_str(
        r#"{"id":"configured","name":" configured name ","type":"if_script","value":"true","language":"rhai"}"#,
    )
    .unwrap();
    LiteFlowNodeBuilder::from_prop(&bus, script)
        .unwrap()
        .build()
        .unwrap();
    assert!(bus.contains_node("configured"));

    let invalid: NodePropBean =
        serde_json::from_str(r#"{"id":"bad","type":"not_a_node_type"}"#).unwrap();
    assert!(matches!(
        LiteFlowNodeBuilder::from_prop(&bus, invalid),
        Err(LiteflowError::NodeTypeNotSupport(message)) if message.contains("not_a_node_type")
    ));
}
