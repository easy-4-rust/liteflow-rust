//! Java `ScriptExecutor` 与 `ScriptExecuteWrap` 运行时语义测试。

use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use liteflow_core::core::NodeComponent;
use liteflow_core::el::NodeRef;
use liteflow_core::exception::LFResult;
use liteflow_core::script::{RhaiScriptExecutor, ScriptExecuteWrap, ScriptExecutor};
use liteflow_core::slot::{CmpContext, DataBus, Frame, Slot};
use serde_json::{Value, json};

/// 用于验证 `ScriptExecuteWrap#getCmp/setCmp` 的脚本组件。
///
/// 对应 Java 测试中放入 `ScriptExecuteWrap.cmp` 的 `NodeComponent`。
struct WrappedComponent;

#[async_trait]
impl NodeComponent for WrappedComponent {
    async fn process(&self, _context: &CmpContext) -> LFResult<Value> {
        Ok(Value::Null)
    }

    fn name(&self) -> &str {
        "wrapped-component"
    }
}

fn script_context() -> (CmpContext, usize) {
    let slot = Arc::new(Slot::new(
        "script-request".to_string(),
        "main-chain",
        json!({"order": 7}),
    ));
    slot.insert_context_bean("profile", Arc::new(json!({"customer": "Ada", "level": 3})));
    slot.set_chain_req_data("sub-chain", json!({"sub": 9}));
    let slot_index = DataBus::offer_slot(Arc::clone(&slot));

    let mut node = NodeRef::new("script-node");
    node.tag = Some("blue".to_string());
    node.data = Some(r#"{"limit":2}"#.to_string());
    let frame = Frame::root()
        .with_current_chain_id("sub-chain")
        .push(4, Some(json!({"sku": "A"})));
    (
        CmpContext {
            inner: slot,
            node,
            frame,
        },
        slot_index,
    )
}

#[test]
fn script_execute_wrap_java_accessors_preserve_the_real_context_snapshot() {
    let (context, slot_index) = script_context();
    let mut execute_wrap = ScriptExecuteWrap::from_context(&context);
    let component: Arc<dyn NodeComponent> = Arc::new(WrappedComponent);
    execute_wrap.set_cmp(Some(Arc::clone(&component)));

    assert_eq!(execute_wrap.get_slot_index(), context.slot_index());
    assert_eq!(execute_wrap.get_curr_chain_id(), "sub-chain");
    #[allow(deprecated)]
    {
        assert_eq!(execute_wrap.get_curr_chain_name(), "sub-chain");
    }
    assert_eq!(execute_wrap.get_node_id(), "script-node");
    assert_eq!(execute_wrap.get_tag(), Some("blue"));
    assert_eq!(execute_wrap.get_cmp_data(), Some(r#"{"limit":2}"#));
    assert_eq!(execute_wrap.get_loop_index(), Some(4));
    assert_eq!(execute_wrap.get_loop_object(), Some(&json!({"sku": "A"})));
    assert_eq!(
        execute_wrap
            .get_cmp()
            .as_ref()
            .map(|component| component.name()),
        Some("wrapped-component")
    );

    execute_wrap.set_cmp(None);
    assert!(execute_wrap.get_cmp().is_none());
    assert!(DataBus::release_slot(slot_index));
}

#[test]
fn script_executor_java_cache_and_compile_entries_use_the_real_rhai_engine() {
    let executor = RhaiScriptExecutor::new();

    <RhaiScriptExecutor as ScriptExecutor>::compile(&executor, "40 + 2").unwrap();
    assert!(<RhaiScriptExecutor as ScriptExecutor>::compile(&executor, "let value = ").is_err());

    executor.load("b", "2").unwrap();
    executor.load("a", "1").unwrap();
    assert_eq!(
        executor.get_node_ids().unwrap(),
        vec!["a".to_string(), "b".to_string()]
    );
    executor.un_load("a").unwrap();
    assert_eq!(executor.get_node_ids().unwrap(), vec!["b".to_string()]);
}

#[test]
fn bind_param_drives_meta_and_context_beans_in_real_rhai_execution() {
    let (context, slot_index) = script_context();
    let executor = RhaiScriptExecutor::new();
    let bindings = executor.bind_param(&context);
    let meta = bindings
        .get("_meta")
        .and_then(Value::as_object)
        .expect("_meta should be an object");

    assert_eq!(
        bindings.get("profile"),
        Some(&json!({"customer": "Ada", "level": 3}))
    );
    assert_eq!(meta.get("currChainId"), Some(&json!("sub-chain")));
    assert_eq!(meta.get("requestData"), Some(&json!({"order": 7})));
    assert_eq!(meta.get("subRequestData"), Some(&json!({"sub": 9})));
    assert_eq!(meta.get("cmpData"), Some(&json!({"limit": 2})));
    assert_eq!(meta.get("loopIndex"), Some(&json!(4)));

    executor
        .load(
            "script-node",
            r#"
                data["customer"] = profile.customer;
                data["order"] = _meta.requestData.order;
                data["sub"] = _meta.subRequestData.sub;
                data["chain"] = _meta.currChainId;
                data["loop"] = _meta.loopObject.sku;
                42
            "#,
        )
        .unwrap();

    assert_eq!(
        executor.execute("script-node", &context).unwrap(),
        json!(42)
    );
    assert_eq!(
        context.inner.data.get("customer").as_deref(),
        Some(&json!("Ada"))
    );
    assert_eq!(context.inner.data.get("order").as_deref(), Some(&json!(7)));
    assert_eq!(context.inner.data.get("sub").as_deref(), Some(&json!(9)));
    assert_eq!(
        context.inner.data.get("chain").as_deref(),
        Some(&json!("sub-chain"))
    );
    assert_eq!(context.inner.data.get("loop").as_deref(), Some(&json!("A")));
    assert!(DataBus::release_slot(slot_index));
}

#[test]
fn serde_context_bean_getters_and_setters_mutate_the_same_slot_object() {
    let (context, slot_index) = script_context();
    let order = Arc::new(RwLock::new(json!({
        "orderNo": null,
        "orderType": 0
    })));
    context.inner.insert_context_bean("order", order.clone());
    let executor = RhaiScriptExecutor::new();
    executor
        .load(
            "script-node",
            r#"
                script_context_call(_script_beans, "order", "setOrderNo", ["order1"]);
                script_context_call(_script_beans, "order", "setOrderType", [7]);
                data["order_no"] =
                    script_context_call(_script_beans, "order", "getOrderNo", []);
                data["order_type"] =
                    script_context_call(_script_beans, "order", "getOrderType", []);
            "#,
        )
        .unwrap();

    executor.execute_script("script-node", &context).unwrap();

    assert_eq!(
        *order.read().unwrap(),
        json!({"orderNo": "order1", "orderType": 7})
    );
    assert_eq!(context.get_data("order_no"), Some(json!("order1")));
    assert_eq!(context.get_data("order_type"), Some(json!(7)));
    assert!(DataBus::release_slot(slot_index));
}
