//! 脚本插件真实运行时验证。
//!
//! 每个测试都通过 `ScriptExecutorFactory -> FlowBus -> FlowExecutor` 完整链路执行，
//! 不直接调用插件内部实现，确保插件注册、节点构建、返回类型校验和 data 写回
//! 都处于验证范围内。

#[cfg(feature = "qlexpress")]
use std::sync::{Arc, Mutex};

use liteflow_core::FlowBus;
use liteflow_core::core::cmp;
#[cfg(feature = "qlexpress")]
use liteflow_core::script::ScriptBeanManager;
use liteflow_core::script::ScriptKind;
#[cfg(feature = "qlexpress")]
use liteflow_core::script::proxy::{ScriptBeanProxy, ScriptMethodProxy};
use serde_json::{Value, json};

fn register_branch_nodes(bus: &FlowBus) {
    bus.register(
        "pass",
        cmp(|ctx| async move {
            ctx.set_data("branch", json!("pass"));
            Ok(Value::Null)
        }),
    );
    bus.register(
        "fail",
        cmp(|ctx| async move {
            ctx.set_data("branch", json!("fail"));
            Ok(Value::Null)
        }),
    );
}

/// 对应 Java LuaScriptExecutor：验证 mlua 真引擎、布尔返回与共享 data 写回。
#[cfg(feature = "lua")]
#[tokio::test]
async fn lua_executes_real_engine_and_writes_data_back() {
    liteflow_script_plugin::register_all().unwrap();
    let bus = FlowBus::new();
    register_branch_nodes(&bus);
    bus.register_script_typed(
        "lua_check",
        "lua",
        ScriptKind::Boolean,
        "data.score = input.score; return input.score >= 60",
    )
    .unwrap();
    bus.add_chain("lua_chain", "IF(lua_check, pass, fail)")
        .unwrap();

    let response = bus
        .execute_with_data("lua_chain", json!({"score": 80}))
        .await;

    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(response.data("score"), Some(json!(80)));
    assert_eq!(response.data("branch"), Some(json!("pass")));
}

/// 独立插件仍复用 core ScriptKind 的强类型返回约束。
#[cfg(feature = "lua")]
#[tokio::test]
async fn lua_rejects_invalid_boolean_result() {
    liteflow_script_plugin::register_all().unwrap();
    let bus = FlowBus::new();
    bus.register_script_typed(
        "lua_bad",
        "lua",
        ScriptKind::Boolean,
        "return 'not-a-boolean'",
    )
    .unwrap();
    bus.register("pass", cmp(|_| async { Ok(Value::Null) }));
    bus.add_chain("lua_bad_chain", "IF(lua_bad, pass)").unwrap();

    let response = bus.execute("lua_bad_chain").await;

    assert!(!response.is_success());
    assert!(
        response
            .cause
            .as_deref()
            .is_some_and(|cause| cause.contains("should return boolean")),
        "{:?}",
        response.cause
    );
}

/// 对应 Java JavaScriptExecutor：验证 Boa 真引擎及 JavaScript 函数体语义。
#[cfg(feature = "javascript")]
#[tokio::test]
async fn javascript_executes_boa_and_writes_data_back() {
    liteflow_script_plugin::register_all().unwrap();
    let bus = FlowBus::new();
    register_branch_nodes(&bus);
    bus.register_script_typed(
        "js_check",
        "javascript",
        ScriptKind::Boolean,
        "data.score = input.score; return input.score >= 60;",
    )
    .unwrap();
    bus.add_chain("js_chain", "IF(js_check, pass, fail)")
        .unwrap();

    let response = bus
        .execute_with_data("js_chain", json!({"score": 61}))
        .await;

    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(response.data("score"), Some(json!(61)));
    assert_eq!(response.data("branch"), Some(json!("pass")));
}

/// GraalJS 兼容入口同样通过 Boa 真引擎执行，但不开放 GraalVM 宿主互操作。
#[cfg(feature = "graaljs")]
#[tokio::test]
async fn graaljs_entrypoint_executes_sandboxed_ecmascript() {
    liteflow_script_plugin::register_all().unwrap();
    let bus = FlowBus::new();
    register_branch_nodes(&bus);
    bus.register_script_typed(
        "graaljs_check",
        "graaljs",
        ScriptKind::Boolean,
        "data.runtime = 'boa'; return input.score >= 60;",
    )
    .unwrap();
    bus.add_chain("graaljs_chain", "IF(graaljs_check, pass, fail)")
        .unwrap();

    let response = bus
        .execute_with_data("graaljs_chain", json!({"score": 88}))
        .await;

    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(response.data("runtime"), Some(json!("boa")));
    assert_eq!(response.data("branch"), Some(json!("pass")));
}

/// 对应 Java PythonScriptExecutor：验证嵌入式 CPython 与顶层 return 改写。
#[cfg(feature = "python")]
#[tokio::test]
async fn python_executes_cpython_and_writes_data_back() {
    liteflow_script_plugin::register_all().unwrap();
    let bus = FlowBus::new();
    register_branch_nodes(&bus);
    bus.register_script_typed(
        "python_check",
        "python",
        ScriptKind::Boolean,
        "data['score'] = input['score']\nreturn input['score'] >= 60",
    )
    .unwrap();
    bus.add_chain("python_chain", "IF(python_check, pass, fail)")
        .unwrap();

    let response = bus
        .execute_with_data("python_chain", json!({"score": 75}))
        .await;

    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(response.data("score"), Some(json!(75)));
    assert_eq!(response.data("branch"), Some(json!("pass")));
}

/// 对应 Java QLExpressScriptExecutor：验证独立 QLExpress 语句解析、条件分支和上下文写回。
#[cfg(feature = "qlexpress")]
#[tokio::test]
async fn qlexpress_executes_java_liteflow_syntax_without_rhai_translation() {
    #[derive(Default)]
    struct OrderContext {
        order_type: Mutex<i64>,
    }

    liteflow_script_plugin::register_all().unwrap();
    ScriptBeanManager::add_script_bean(ScriptBeanProxy::new(
        "ql_math",
        &["double"],
        &[],
        [ScriptMethodProxy::new(
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
        )],
    ));
    let order_context = Arc::new(OrderContext::default());
    let order_context_for_setter = Arc::clone(&order_context);
    let order_proxy = ScriptBeanProxy::new(
        "order",
        &["setOrderType", "getOrderType"],
        &[],
        [
            ScriptMethodProxy::new(
                "setOrderType",
                Arc::new(move |arguments| {
                    let order_type =
                        arguments.first().and_then(Value::as_i64).ok_or_else(|| {
                            liteflow_core::LiteflowError::Script {
                                node: "ql_common".to_string(),
                                msg: "order type must be an integer".to_string(),
                            }
                        })?;
                    *order_context_for_setter.order_type.lock().unwrap() = order_type;
                    Ok(Value::Null)
                }),
            ),
            ScriptMethodProxy::new(
                "getOrderType",
                Arc::new({
                    let order_context = Arc::clone(&order_context);
                    move |_| Ok(json!(*order_context.order_type.lock().unwrap()))
                }),
            ),
        ],
    );
    let bus = FlowBus::new();
    register_branch_nodes(&bus);
    bus.register(
        "loop_body",
        cmp(|ctx| async move {
            let count = ctx.get_data_as::<i64>("loop_count").unwrap_or_default();
            ctx.set_data("loop_count", json!(count + 1));
            Ok(Value::Null)
        }),
    );
    bus.register_script(
        "ql_common",
        "qlexpress",
        r#"
            a = 3;
            b = 2;
            defaultContext.setData("score", a * b + 84);
            answer = ql_math.double(21);
            defaultContext.setData("answer", answer);
            order.setOrderType(a * b);
            order_type = order.getOrderType();
            defaultContext.setData("order_type", order_type);
            node_id = _meta.get("nodeId");
            defaultContext.setData("node_id", node_id);
        "#,
    )
    .unwrap();
    bus.register_script_typed(
        "ql_check",
        "qlexpress",
        ScriptKind::Boolean,
        r#"
            score = defaultContext.getData("score");
            if (score >= 60) {
                return true;
            } else {
                return false;
            }
        "#,
    )
    .unwrap();
    bus.register_script_typed(
        "ql_route",
        "qlexpress",
        ScriptKind::Switch,
        r#"
            score = defaultContext.getData("score");
            if (score > 100) {
                return "fail";
            } else {
                return "pass";
            }
        "#,
    )
    .unwrap();
    bus.register_script_typed("ql_count", "qlexpress", ScriptKind::For, "return 3;")
        .unwrap();
    bus.add_chain(
        "ql_chain",
        "THEN(ql_common, IF(ql_check, pass, fail), SWITCH(ql_route).to(pass, fail), FOR(ql_count).DO(loop_body))",
    )
    .unwrap();

    let response = bus
        .execute_with(
            "ql_chain",
            Value::Null,
            vec![
                ("order".to_string(), Arc::new(order_proxy)),
                ("order_state".to_string(), order_context),
            ],
        )
        .await;

    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(response.data("score"), Some(json!(90)));
    assert_eq!(response.data("answer"), Some(json!(42)));
    assert_eq!(response.data("order_type"), Some(json!(6)));
    assert_eq!(
        *response
            .bean::<OrderContext>("order_state")
            .unwrap()
            .order_type
            .lock()
            .unwrap(),
        6
    );
    assert_eq!(response.data("node_id"), Some(json!("ql_common")));
    assert_eq!(response.data("branch"), Some(json!("pass")));
    assert_eq!(response.data("loop_count"), Some(json!(3)));
    ScriptBeanManager::remove_script_bean("ql_math");
}

/// Aviator/Groovy 映射只接受与 Rhai 重叠的表达式语法，并实际经过完整执行链。
#[cfg(all(feature = "aviator", feature = "groovy"))]
#[tokio::test]
async fn jvm_expression_subcrates_execute_declared_common_subset() {
    liteflow_script_plugin::register_all().unwrap();
    for (language, node_id, chain_id) in [
        ("aviator", "aviator_check", "aviator_chain"),
        ("groovy", "groovy_check", "groovy_chain"),
    ] {
        let bus = FlowBus::new();
        register_branch_nodes(&bus);
        bus.register_script_typed(
            node_id,
            language,
            ScriptKind::Boolean,
            "return input.score >= 60;",
        )
        .unwrap();
        bus.add_chain(chain_id, &format!("IF({node_id}, pass, fail)"))
            .unwrap();

        let response = bus.execute_with_data(chain_id, json!({"score": 66})).await;
        assert!(response.is_success(), "{:?}", response.cause);
        assert_eq!(response.data("branch"), Some(json!("pass")));
    }
}
