use std::sync::Arc;
use std::sync::atomic::Ordering;

use liteflow_core::el::NodeRef;
use liteflow_core::slot::{CmpContext, Frame, Slot};
use liteflow_core::{LiteflowError, ScriptCommonComponent};
use serde_json::{Value, json};

fn context() -> CmpContext {
    CmpContext {
        inner: Arc::new(Slot::new(
            "script-common-request".to_string(),
            "script-common-chain",
            Value::Null,
        )),
        node: NodeRef::new("script-common"),
        frame: Frame::root().with_current_chain_id("script-common-chain"),
    }
}

/// 验证普通脚本组件的 Java 命名入口共享真实 Rhai 缓存与执行钩子。
#[tokio::test]
async fn script_common_component_java_entries_drive_real_script_execution() {
    let component = ScriptCommonComponent::new("script-common", "40 + 2").unwrap();
    let context = context();

    assert_eq!(component.process(&context).await.unwrap(), json!(42));
    component.load_script("43", "rhai").unwrap();
    assert_eq!(component.process(&context).await.unwrap(), json!(43));
    assert!(component.load_script("44", "unknown").is_err());
    assert!(component.is_access(&context));
    assert!(!component.is_continue_on_error(&context));
    assert!(!component.is_end(&context));

    component.before_process(&context).await.unwrap();
    component.on_success(&context).await.unwrap();
    component
        .on_error(&context, &LiteflowError::Custom("expected".to_string()))
        .await;
    component.after_process(&context).await;
    component.rollback(&context).await.unwrap();

    context.inner.ended.store(true, Ordering::Release);
    assert!(component.is_end(&context));
}
