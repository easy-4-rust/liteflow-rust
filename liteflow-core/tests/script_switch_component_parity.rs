use std::sync::Arc;

use liteflow_core::el::NodeRef;
use liteflow_core::slot::{CmpContext, Frame, Slot};
use liteflow_core::{NodeComponent, ScriptSwitchComponent};
use serde_json::{Value, json};

fn context() -> CmpContext {
    CmpContext {
        inner: Arc::new(Slot::new(
            "script-switch-request".to_string(),
            "script-switch-chain",
            Value::Null,
        )),
        node: NodeRef::new("script-switch"),
        frame: Frame::root().with_current_chain_id("script-switch-chain"),
    }
}

/// 验证 SWITCH 脚本保留字符串、标签及 nullable 返回语义。
#[tokio::test]
async fn script_switch_component_java_entries_preserve_nullable_string_contract() {
    let component = ScriptSwitchComponent::new("script-switch", r#""target:blue""#).unwrap();
    let context = context();

    assert_eq!(
        component.process_switch(&context).await.unwrap(),
        Some("target:blue".to_string())
    );
    assert_eq!(
        NodeComponent::process(&component, &context).await.unwrap(),
        json!("target:blue")
    );
    component.load_script("()", "rhai").unwrap();
    assert_eq!(component.process_switch(&context).await.unwrap(), None);
    assert_eq!(
        NodeComponent::process(&component, &context).await.unwrap(),
        Value::Null
    );
    component.rollback(&context).await.unwrap();
}
