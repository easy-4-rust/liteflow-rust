use std::sync::Arc;

use liteflow_core::el::NodeRef;
use liteflow_core::slot::{CmpContext, Frame, Slot};
use liteflow_core::{NodeComponent, ScriptBooleanComponent};
use serde_json::{Value, json};

fn context() -> CmpContext {
    CmpContext {
        inner: Arc::new(Slot::new(
            "script-boolean-request".to_string(),
            "script-boolean-chain",
            Value::Null,
        )),
        node: NodeRef::new("script-boolean"),
        frame: Frame::root().with_current_chain_id("script-boolean-chain"),
    }
}

/// 验证布尔脚本专用返回值与 NodeComponent 主执行入口使用同一编译缓存。
#[tokio::test]
async fn script_boolean_component_java_entries_preserve_boolean_contract() {
    let component = ScriptBooleanComponent::new("script-boolean", "40 > 2").unwrap();
    let context = context();

    assert!(component.process_boolean(&context).await.unwrap());
    assert_eq!(
        NodeComponent::process(&component, &context).await.unwrap(),
        json!(true)
    );
    component.load_script("false", "rhai").unwrap();
    assert!(!component.process_boolean(&context).await.unwrap());
    assert!(component.is_access(&context));
    assert!(!component.is_continue_on_error(&context));
    component.rollback(&context).await.unwrap();
}
