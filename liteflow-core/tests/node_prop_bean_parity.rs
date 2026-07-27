use liteflow_core::NodePropBean;

/// 验证 NodePropBean 的 Java Bean 方法名、链式 setter 与 serde 别名共同工作。
#[test]
fn node_prop_bean_exposes_java_equivalent_accessors_and_jackson_aliases() {
    let bean: NodePropBean = serde_json::from_value(serde_json::json!({
        "id": "script_node",
        "name": "脚本节点",
        "class": "demo.ScriptComponent",
        "value": "return 1",
        "type": "script",
        "file": "rules/demo.ql",
        "language": "qlexpress"
    }))
    .expect("Java 规则字段应能反序列化为 NodePropBean");

    assert_eq!(bean.get_id(), Some("script_node"));
    assert_eq!(bean.get_name(), Some("脚本节点"));
    assert_eq!(bean.get_clazz(), Some("demo.ScriptComponent"));
    assert_eq!(bean.get_script(), Some("return 1"));
    assert_eq!(bean.get_type(), Some("script"));
    assert_eq!(bean.get_file(), Some("rules/demo.ql"));
    assert_eq!(bean.get_language(), Some("qlexpress"));

    let updated = NodePropBean::default()
        .set_id("n1")
        .set_name("普通节点")
        .set_clazz("demo.CommonComponent")
        .set_script("a = 1")
        .set_type("common")
        .set_file("rules/common.ql")
        .set_language("qlexpress");
    assert_eq!(updated.get_id(), Some("n1"));
    assert_eq!(updated.get_name(), Some("普通节点"));
    assert_eq!(updated.get_clazz(), Some("demo.CommonComponent"));
    assert_eq!(updated.get_script(), Some("a = 1"));
    assert_eq!(updated.get_type(), Some("common"));
    assert_eq!(updated.get_file(), Some("rules/common.ql"));
    assert_eq!(updated.get_language(), Some("qlexpress"));
}
