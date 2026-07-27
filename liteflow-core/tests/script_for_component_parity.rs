use std::sync::Arc;

use liteflow_core::el::NodeRef;
use liteflow_core::slot::{CmpContext, Frame, Slot};
use liteflow_core::{NodeComponent, ScriptForComponent};
use serde_json::{Value, json};

fn context() -> CmpContext {
    CmpContext {
        inner: Arc::new(Slot::new(
            "script-for-request".to_string(),
            "script-for-chain",
            Value::Null,
        )),
        node: NodeRef::new("script-for"),
        frame: Frame::root().with_current_chain_id("script-for-chain"),
    }
}

/// 验证 FOR 脚本只接受非负整数并进入统一 NodeComponent 返回值。
#[tokio::test]
async fn script_for_component_java_entries_preserve_integer_contract() {
    let component = ScriptForComponent::new("script-for", "3").unwrap();
    let context = context();

    assert_eq!(component.process_for(&context).await.unwrap(), 3);
    assert_eq!(
        NodeComponent::process(&component, &context).await.unwrap(),
        json!(3)
    );
    component.load_script("-1", "rhai").unwrap();
    assert!(component.process_for(&context).await.is_err());
    component.load_script("5", "rhai").unwrap();
    assert_eq!(component.process_for(&context).await.unwrap(), 5);
    component.rollback(&context).await.unwrap();
}
