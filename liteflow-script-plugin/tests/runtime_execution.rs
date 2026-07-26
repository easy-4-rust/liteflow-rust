//! 脚本插件真实运行时验证。
//!
//! 每个测试都通过 `ScriptExecutorFactory -> FlowBus -> FlowExecutor` 完整链路执行，
//! 不直接调用插件内部实现，确保插件注册、节点构建、返回类型校验和 data 写回
//! 都处于验证范围内。

use liteflow_core::FlowBus;
use liteflow_core::core::cmp;
use liteflow_core::script::ScriptKind;
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

/// JVM 表达式生态仅验证明确声明的 Rhai 公共子集，不声称兼容 JVM 专属语法。
#[cfg(feature = "qlexpress")]
#[tokio::test]
async fn qlexpress_common_expression_subset_executes() {
    liteflow_script_plugin::register_all().unwrap();
    let bus = FlowBus::new();
    register_branch_nodes(&bus);
    bus.register_script_typed(
        "ql_check",
        "qlexpress",
        ScriptKind::Boolean,
        "return input.score >= 60;",
    )
    .unwrap();
    bus.add_chain("ql_chain", "IF(ql_check, pass, fail)")
        .unwrap();

    let response = bus
        .execute_with_data("ql_chain", json!({"score": 90}))
        .await;

    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(response.data("branch"), Some(json!("pass")));
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
