//! Java `ScriptExecutor` 与 `ScriptExecuteWrap` 运行时语义测试。

use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use liteflow_core::core::NodeComponent;
use liteflow_core::el::NodeRef;
use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::script::proxy::{ScriptBeanProxy, ScriptMethodProxy};
use liteflow_core::script::{
    RhaiScriptExecutor, ScriptBeanManager, ScriptExecuteWrap, ScriptExecutor,
};
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
    let executor = RhaiScriptExecutor::default();

    <RhaiScriptExecutor as ScriptExecutor>::compile(&executor, "40 + 2").unwrap();
    assert!(<RhaiScriptExecutor as ScriptExecutor>::compile(&executor, "let value = ").is_err());
    assert!(executor.validate("40 + 2"));
    assert!(!executor.validate("let value = "));
    assert!(executor.validate_with_ex("40 + 2").is_success());
    assert!(!executor.validate_with_ex("let value = ").is_success());
    assert_eq!(
        executor.script_type(),
        liteflow_core::enums::ScriptTypeEnum::Rhai
    );

    executor.load("b", "2").unwrap();
    executor.load("a", "1").unwrap();
    assert_eq!(
        executor.get_node_ids().unwrap(),
        vec!["a".to_string(), "b".to_string()]
    );
    executor.un_load("a").unwrap();
    assert_eq!(executor.get_node_ids().unwrap(), vec!["b".to_string()]);
    executor.unload("not-loaded").unwrap();
    executor.clean_cache().unwrap();
    assert!(executor.node_ids().unwrap().is_empty());
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

#[test]
fn runtime_bridges_cover_request_data_script_beans_and_language_adapters() {
    let (context, slot_index) = script_context();
    context.set_data("existing", json!(9));

    let local_method = ScriptMethodProxy::new(
        "echo",
        Arc::new(|arguments| Ok(arguments.first().cloned().unwrap_or(Value::Null))),
    );
    context.inner.insert_context_bean(
        "local_bridge",
        Arc::new(ScriptBeanProxy::new(
            "local_bridge",
            &["echo"],
            &[],
            [local_method],
        )),
    );

    let global_method = ScriptMethodProxy::new(
        "double",
        Arc::new(|arguments| {
            Ok(json!(
                arguments
                    .first()
                    .and_then(Value::as_i64)
                    .unwrap_or_default()
                    * 2
            ))
        }),
    );
    ScriptBeanManager::add_script_bean(ScriptBeanProxy::new(
        "global_bridge",
        &["double"],
        &[],
        [global_method],
    ));

    let acronym_bean = Arc::new(RwLock::new(json!({"URL": "before"})));
    context
        .inner
        .insert_context_bean("acronym", acronym_bean.clone());

    let executor = RhaiScriptExecutor::new();
    executor
        .load(
            "bridge-node",
            r#"
                data["local"] =
                    script_context_call(_script_beans, "local_bridge", "echo", ["ok"]);
                data["global"] =
                    script_context_call(_script_beans, "global_bridge", "double", [21]);
                data["existing_before"] = script_data_get(_script_data, "existing");
                data["has_existing"] = script_data_has(_script_data, "existing");
                data["has_missing"] = script_data_has(_script_data, "missing");
                script_data_set(_script_data, "written", 42);
                data["written_during_script"] = script_data_get(_script_data, "written");
                data["has_written"] = script_data_has(_script_data, "written");
                data["number_int"] = kotlin_to_int(7);
                data["string_int"] = kotlin_to_int("8");
                data["now"] = aviator_now();
                script_context_call(_script_beans, "acronym", "setURL", ["after"]);
                data["url"] =
                    script_context_call(_script_beans, "acronym", "getURL", []);
            "#,
        )
        .unwrap();

    executor.execute_script("bridge-node", &context).unwrap();
    assert_eq!(context.get_data("local"), Some(json!("ok")));
    assert_eq!(context.get_data("global"), Some(json!(42)));
    assert_eq!(context.get_data("existing_before"), Some(json!(9)));
    assert_eq!(context.get_data("has_existing"), Some(json!(true)));
    assert_eq!(context.get_data("has_missing"), Some(json!(false)));
    assert_eq!(context.get_data("written_during_script"), Some(json!(42)));
    assert_eq!(context.get_data("written"), Some(json!(42)));
    assert_eq!(context.get_data("has_written"), Some(json!(true)));
    assert_eq!(context.get_data("number_int"), Some(json!(7)));
    assert_eq!(context.get_data("string_int"), Some(json!(8)));
    assert!(
        context
            .get_data("now")
            .and_then(|value| value.as_str().map(str::to_owned))
            .is_some_and(|value| !value.is_empty())
    );
    assert_eq!(context.get_data("url"), Some(json!("after")));
    assert_eq!(*acronym_bean.read().unwrap(), json!({"URL": "after"}));

    ScriptBeanManager::remove_script_bean("global_bridge");
    assert!(DataBus::release_slot(slot_index));
}

#[test]
fn runtime_bridge_failures_preserve_java_business_errors_and_diagnostics() {
    let (context, slot_index) = script_context();
    let non_object = Arc::new(RwLock::new(json!([1, 2])));
    context.inner.insert_context_bean("non_object", non_object);
    let bean = Arc::new(RwLock::new(json!({"value": 1})));
    context.inner.insert_context_bean("bean", bean);

    let executor = RhaiScriptExecutor::new();
    let failures = [
        (
            "missing-global",
            r#"script_call("missing_global", "run", []);"#,
            "missing_global",
        ),
        (
            "missing-context",
            r#"script_context_call(_script_beans, "missing_context", "run", []);"#,
            "missing_context",
        ),
        (
            "non-object",
            r#"script_context_call(_script_beans, "non_object", "getValue", []);"#,
            "must be a JSON object",
        ),
        (
            "setter-arity",
            r#"script_context_call(_script_beans, "bean", "setValue", [1, 2]);"#,
            "requires 1 argument",
        ),
        (
            "getter-arity",
            r#"script_context_call(_script_beans, "bean", "getValue", [1]);"#,
            "requires 0 arguments",
        ),
        (
            "unknown-method",
            r#"script_context_call(_script_beans, "bean", "computeValue", []);"#,
            "outside the serde JavaBean",
        ),
        (
            "lowercase-property",
            r#"script_context_call(_script_beans, "bean", "getvalue", []);"#,
            "outside the serde JavaBean",
        ),
        ("string-int", r#"kotlin_to_int("bad");"#, "toInt failed"),
        (
            "number-int",
            r#"kotlin_to_int(1.5);"#,
            "conversion overflow",
        ),
        ("type-int", r#"kotlin_to_int(true);"#, "does not accept"),
        ("ordinary-eval", r#"let value = 1 / 0;"#, "eval error"),
    ];

    for (node_id, script, expected) in failures {
        executor.load(node_id, script).unwrap();
        let error = executor.execute_script(node_id, &context).unwrap_err();
        assert!(
            error.to_string().contains(expected),
            "{node_id}: expected {expected:?}, got {error}"
        );
    }

    executor
        .load("business-message", r#"liteflow_throw("business failed");"#)
        .unwrap();
    match executor
        .execute_script("business-message", &context)
        .unwrap_err()
    {
        LiteflowError::LiteFlow(error) => {
            assert_eq!(error.get_code(), None);
            assert_eq!(error.get_message(), "business failed");
        }
        error => panic!("应还原为 LiteFlowException，实际为 {error:?}"),
    }

    executor
        .load(
            "business-code",
            r#"
                fn fail() {
                    liteflow_throw("BIZ-42", "coded failure");
                }
                fail();
            "#,
        )
        .unwrap();
    match executor
        .execute_script("business-code", &context)
        .unwrap_err()
    {
        LiteflowError::LiteFlow(error) => {
            assert_eq!(error.get_code(), Some("BIZ-42"));
            assert_eq!(error.get_message(), "coded failure");
        }
        error => panic!("嵌套函数业务异常应穿透，实际为 {error:?}"),
    }

    let missing = executor
        .execute_script("never-loaded", &context)
        .unwrap_err();
    assert!(missing.to_string().contains("is not loaded"));
    assert!(DataBus::release_slot(slot_index));
}
