use std::sync::Arc;

use async_trait::async_trait;
use liteflow_core::script::annotation::ScriptBean;
use liteflow_core::script::jsr223::JSR223ScriptExecutor;
use liteflow_core::script::proxy::{ScriptBeanProxy, ScriptMethodProxy};
use liteflow_core::script::validator::ScriptValidator;
use liteflow_core::script::{ScriptBeanManager, ScriptExecuteWrap, ScriptKind};
use liteflow_core::{CmpContext, FlowBus, LiteflowError, NodeComponent};
use serde_json::{Value, json};

fn sum_method() -> ScriptMethodProxy {
    ScriptMethodProxy::new(
        "sum",
        Arc::new(|arguments| {
            let left = arguments
                .first()
                .and_then(Value::as_i64)
                .unwrap_or_default();
            let right = arguments.get(1).and_then(Value::as_i64).unwrap_or_default();
            Ok(json!(left + right))
        }),
    )
}

#[tokio::test]
async fn script_bean_proxy_filters_methods_and_rhai_calls_real_function() {
    let metadata = ScriptBean::new("s7_math")
        .include_method_names(["sum", "hidden"])
        .exclude_method_names(["hidden"]);
    let hidden = ScriptMethodProxy::new("hidden", Arc::new(|_| Ok(json!("secret"))));
    let proxy = ScriptBeanProxy::new("s7_math", &metadata, [sum_method(), hidden]);

    assert_eq!(proxy.method_names(), vec!["sum"]);
    assert!(proxy.invoke("hidden", &[]).is_err());
    ScriptBeanManager::add_script_bean(proxy);

    let bus = FlowBus::new();
    bus.register_script(
        "scriptBeanNode",
        "rhai",
        r#"data["sum"] = script_call("s7_math", "sum", [20, 22]);"#,
    )
    .unwrap();
    bus.add_chain("scriptBeanChain", "THEN(scriptBeanNode)")
        .unwrap();

    let response = bus.execute("scriptBeanChain").await;

    assert!(response.is_success(), "{}", response.message);
    assert_eq!(response.data("sum"), Some(json!(42)));
    ScriptBeanManager::remove_script_bean("s7_math");
}

struct CachedScriptComponent {
    executor: Arc<JSR223ScriptExecutor>,
}

#[async_trait]
impl NodeComponent for CachedScriptComponent {
    async fn process(&self, context: &CmpContext) -> Result<Value, LiteflowError> {
        self.executor
            .execute(&ScriptExecuteWrap::from_context(context), context)
            .await
    }
}

#[tokio::test]
async fn jsr223_adapter_executes_cached_component_on_real_flow_context() {
    let executor = Arc::new(JSR223ScriptExecutor::new("rhai", ScriptKind::Common));
    executor.init().unwrap();
    executor
        .load("cachedScript", r#"data["cached"] = node_id; 7"#)
        .unwrap();
    assert_eq!(executor.node_ids(), vec!["cachedScript"]);

    let bus = FlowBus::new();
    bus.register(
        "cachedScript",
        CachedScriptComponent {
            executor: executor.clone(),
        },
    );
    bus.add_chain("cachedScriptChain", "THEN(cachedScript)")
        .unwrap();

    let response = bus.execute("cachedScriptChain").await;

    assert!(response.is_success(), "{}", response.message);
    assert_eq!(response.data("cached"), Some(json!("cachedScript")));
    executor.unload("cachedScript");
    assert!(executor.node_ids().is_empty());
}

#[test]
fn validator_and_execute_wrap_preserve_java_object_contracts() {
    assert!(ScriptValidator::validate_for_language("rhai", "40 + 2"));
    let invalid = ScriptValidator::validate_for_language_with_ex("rhai", "let =");
    assert!(!invalid.is_success());
    assert!(invalid.cause().is_some());

    let mut execute_wrap = ScriptExecuteWrap::default();
    execute_wrap.set_slot_index(Some(3));
    execute_wrap.set_curr_chain_id("orderChain");
    execute_wrap.set_node_id("scriptNode");
    execute_wrap.set_tag(Some("blue".to_string()));
    execute_wrap.set_cmp_data(Some(r#"{"limit":2}"#.to_string()));
    execute_wrap.set_loop_index(Some(4));
    execute_wrap.set_loop_object(Some(json!({"id": 9})));

    assert_eq!(execute_wrap.slot_index(), Some(3));
    assert_eq!(execute_wrap.curr_chain_id(), "orderChain");
    assert_eq!(execute_wrap.node_id(), "scriptNode");
    assert_eq!(execute_wrap.tag(), Some("blue"));
    assert_eq!(execute_wrap.loop_index(), Some(4));
    assert_eq!(execute_wrap.loop_object(), Some(&json!({"id": 9})));
}
