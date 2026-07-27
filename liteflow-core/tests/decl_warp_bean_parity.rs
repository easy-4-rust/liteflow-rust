//! Java `DeclWarpBean` 字段访问语义回归测试。

use std::sync::Arc;

use async_trait::async_trait;
use liteflow_core::core::DeclComponent;
use liteflow_core::core::proxy::DeclWarpBean;
use liteflow_core::{CmpContext, LiteflowError, NodeTypeEnum};
use serde_json::Value;

struct FirstDeclComponent;

#[async_trait]
impl DeclComponent for FirstDeclComponent {
    async fn call(&self, _method: &str, _context: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(Value::Null)
    }
}

struct SecondDeclComponent;

#[async_trait]
impl DeclComponent for SecondDeclComponent {
    async fn call(&self, _method: &str, _context: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(Value::Null)
    }
}

#[test]
fn java_named_getters_and_setters_share_the_real_decl_warp_state() {
    let first: Arc<dyn DeclComponent> = Arc::new(FirstDeclComponent);
    let second: Arc<dyn DeclComponent> = Arc::new(SecondDeclComponent);
    let mut declaration = DeclWarpBean::new(
        "inventory",
        "库存组件",
        NodeTypeEnum::Common,
        Arc::clone(&first),
        "tests::FirstDeclComponent",
        Vec::new(),
    );

    assert_eq!(declaration.get_node_id(), "inventory");
    assert_eq!(declaration.get_node_name(), "库存组件");
    assert_eq!(declaration.get_node_type(), NodeTypeEnum::Common);
    assert!(Arc::ptr_eq(declaration.get_raw_bean(), &first));
    assert_eq!(declaration.get_raw_clazz(), "tests::FirstDeclComponent");
    assert!(declaration.get_method_wrap_bean_list().is_empty());

    declaration.set_node_id("inventory-v2");
    declaration.set_node_name("库存组件二");
    declaration.set_node_type(NodeTypeEnum::Boolean);
    declaration.set_raw_bean(Arc::clone(&second));
    declaration.set_raw_clazz("tests::SecondDeclComponent");
    declaration.set_method_wrap_bean_list(Vec::new());

    assert_eq!(declaration.get_node_id(), "inventory-v2");
    assert_eq!(declaration.get_node_name(), "库存组件二");
    assert_eq!(declaration.get_node_type(), NodeTypeEnum::Boolean);
    assert!(Arc::ptr_eq(declaration.get_raw_bean(), &second));
    assert_eq!(declaration.get_raw_clazz(), "tests::SecondDeclComponent");
}
