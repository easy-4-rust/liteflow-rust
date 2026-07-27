//! Java `NodeConvertHelper.NodeSimpleVO` 解析和访问语义回归测试。

use liteflow_core::parser::helper::NodeConvertHelper;

#[test]
fn colon_script_key_populates_all_java_named_fields() {
    let mut node =
        NodeConvertHelper::convert("scriptNode:script:订单脚本:qlexpress:false").unwrap();

    assert_eq!(node.get_node_id(), "scriptNode");
    assert_eq!(node.get_type(), "script");
    assert_eq!(node.get_name(), "订单脚本");
    assert_eq!(node.get_language(), Some("qlexpress"));
    assert!(!node.get_enable());
    assert_eq!(node.get_script(), None);

    node.set_node_id("scriptNodeV2");
    node.set_type("boolean_script");
    node.set_name("订单判断");
    node.set_language("groovy");
    node.set_enable(true);
    node.set_script("return true");

    assert_eq!(node.get_node_id(), "scriptNodeV2");
    assert_eq!(node.get_type(), "boolean_script");
    assert_eq!(node.get_name(), "订单判断");
    assert_eq!(node.get_language(), Some("groovy"));
    assert!(node.get_enable());
    assert_eq!(node.get_script(), Some("return true"));
}

#[test]
fn incomplete_colon_script_key_is_rejected_like_java_regex() {
    assert!(NodeConvertHelper::convert("scriptNode").is_none());
    assert!(NodeConvertHelper::convert("scriptNode:").is_none());
}
