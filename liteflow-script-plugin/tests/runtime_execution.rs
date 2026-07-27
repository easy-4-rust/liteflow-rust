//! 脚本插件真实运行时验证。
//!
//! 每个测试都通过 `ScriptExecutorFactory -> FlowBus -> FlowExecutor` 完整链路执行，
//! 不直接调用插件内部实现，确保插件注册、节点构建、返回类型校验和 data 写回
//! 都处于验证范围内。

#[cfg(feature = "groovy")]
use std::any::Any;
#[cfg(any(feature = "groovy", feature = "qlexpress"))]
use std::sync::Arc;
#[cfg(feature = "qlexpress")]
use std::sync::Mutex;

use liteflow_core::FlowBus;
use liteflow_core::core::cmp;
#[cfg(any(feature = "groovy", feature = "qlexpress"))]
use liteflow_core::script::ScriptBeanManager;
use liteflow_core::script::ScriptKind;
#[cfg(any(feature = "groovy", feature = "qlexpress"))]
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

/// 对应 Java Aviator 基线：验证导入、DateUtil、println 与 setData 上下文写回。
#[cfg(feature = "aviator")]
#[tokio::test]
async fn aviator_executes_java_liteflow_baseline_syntax() {
    liteflow_script_plugin::register_all().unwrap();
    let bus = FlowBus::new();
    bus.register_script(
        "aviator_common",
        "aviator",
        r#"
            use java.util.Date;
            use cn.hutool.core.date.DateUtil;
            let d = DateUtil.formatDateTime(new Date());
            println(d);
            a = 2;
            b = 3;
            setData(defaultContext, "s1", a*b);
        "#,
    )
    .unwrap();
    bus.add_chain("aviator_baseline_chain", "THEN(aviator_common)")
        .unwrap();

    let response = bus.execute("aviator_baseline_chain").await;

    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(response.data("s1"), Some(json!(6)));
}

/// Aviator/Groovy 公共表达式兼容面经过完整布尔分支执行链。
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

/// 对应 Java Groovy 常用基线：验证 def、DefaultContext 和 if/else。
#[cfg(feature = "groovy")]
#[tokio::test]
async fn groovy_executes_liteflow_context_binding_baseline() {
    liteflow_script_plugin::register_all().unwrap();
    let bus = FlowBus::new();
    register_branch_nodes(&bus);
    bus.register_script(
        "groovy_common",
        "groovy",
        r#"
            def a = 3
            int b = 2
            defaultContext.setData("score", a * b)
        "#,
    )
    .unwrap();
    bus.register_script_typed(
        "groovy_check",
        "groovy",
        ScriptKind::Boolean,
        r#"
            def score = defaultContext.getData("score")
            if (defaultContext.hasData("score")) {
                return score == 6
            } else {
                return false
            }
        "#,
    )
    .unwrap();
    bus.add_chain(
        "groovy_baseline_chain",
        "THEN(groovy_common, IF(groovy_check, pass, fail))",
    )
    .unwrap();

    let response = bus.execute("groovy_baseline_chain").await;

    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(response.data("score"), Some(json!(6)));
    assert_eq!(response.data("branch"), Some(json!("pass")));
}

/// 对应 Java Groovy ScriptBean 测试：直接对象方法调用必须经过 include/exclude 代理规则。
#[cfg(feature = "groovy")]
#[tokio::test]
async fn groovy_invokes_controlled_script_bean_and_rejects_excluded_method() {
    liteflow_script_plugin::register_all().unwrap();
    ScriptBeanManager::add_script_bean(ScriptBeanProxy::new(
        "groovyGreeting",
        &["sayHello"],
        &[],
        [ScriptMethodProxy::new(
            "sayHello",
            Arc::new(|arguments| {
                let name = arguments
                    .first()
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                Ok(json!(format!("hello,{name}")))
            }),
        )],
    ));
    ScriptBeanManager::add_script_bean(ScriptBeanProxy::new(
        "groovyDenied",
        &[],
        &["sayHello"],
        [ScriptMethodProxy::new(
            "sayHello",
            Arc::new(|_| Ok(json!("must not execute"))),
        )],
    ));

    let success_bus = FlowBus::new();
    success_bus
        .register_script(
            "groovy_script_bean",
            "groovy",
            r#"defaultContext.setData("demo", groovyGreeting.sayHello("kobe"))"#,
        )
        .unwrap();
    success_bus
        .add_chain("groovy_script_bean_chain", "groovy_script_bean")
        .unwrap();

    let success = success_bus.execute("groovy_script_bean_chain").await;
    assert!(success.is_success(), "{:?}", success.cause);
    assert_eq!(success.data("demo"), Some(json!("hello,kobe")));

    let denied_bus = FlowBus::new();
    denied_bus
        .register_script(
            "groovy_denied_bean",
            "groovy",
            r#"defaultContext.setData("demo", groovyDenied.sayHello("kobe"))"#,
        )
        .unwrap();
    denied_bus
        .add_chain("groovy_denied_bean_chain", "groovy_denied_bean")
        .unwrap();

    let denied = denied_bus.execute("groovy_denied_bean_chain").await;
    assert!(!denied.is_success());
    assert!(
        denied
            .cause
            .as_deref()
            .is_some_and(|cause| cause.contains("not exposed"))
    );

    ScriptBeanManager::remove_script_bean("groovyGreeting");
    ScriptBeanManager::remove_script_bean("groovyDenied");
}

/// 对应 Java execute2Resp 的 contextBeanArray：执行级代理优先于全局 ScriptBean。
#[cfg(feature = "groovy")]
#[tokio::test]
async fn groovy_invokes_per_execution_context_script_bean_without_global_state() {
    liteflow_script_plugin::register_all().unwrap();
    let bus = FlowBus::new();
    bus.register_script(
        "groovy_context_bean",
        "groovy",
        r#"defaultContext.setData("demo", slotGreeting.sayHello("jordan"))"#,
    )
    .unwrap();
    bus.add_chain("groovy_context_bean_chain", "groovy_context_bean")
        .unwrap();

    let proxy = ScriptBeanProxy::new(
        "slotGreeting",
        &["sayHello"],
        &[],
        [ScriptMethodProxy::new(
            "sayHello",
            Arc::new(|arguments| {
                let name = arguments
                    .first()
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                Ok(json!(format!("hello,{name}")))
            }),
        )],
    );
    let beans: Vec<(String, Arc<dyn Any + Send + Sync>)> =
        vec![("slotGreeting".to_string(), Arc::new(proxy))];

    let response = bus
        .execute_with("groovy_context_bean_chain", Value::Null, beans)
        .await;

    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(response.data("demo"), Some(json!("hello,jordan")));
    assert!(ScriptBeanManager::get_script_bean("slotGreeting").is_none());
}

/// 对应 Java Groovy cmpdata/flow.xml：验证 `_meta.cmpData` 的对象字段访问。
#[cfg(feature = "groovy")]
#[tokio::test]
async fn groovy_reads_structured_cmp_data_and_supports_println_statement() {
    liteflow_script_plugin::register_all().unwrap();
    let bus = FlowBus::new();
    bus.register_script(
        "groovy_cmp_data",
        "groovy",
        r#"
            def birth = _meta.cmpData.birth
            println birth
            defaultContext.setData("birth", birth)
        "#,
    )
    .unwrap();
    bus.add_chain(
        "groovy_cmp_data_chain",
        r#"groovy_cmp_data.data('{"name":"jack","birth":"1995-10-01"}')"#,
    )
    .unwrap();

    let response = bus.execute("groovy_cmp_data_chain").await;

    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(response.data("birth"), Some(json!("1995-10-01")));
}

/// 对应 Java Groovy loop/flow.xml：脚本驱动 FOR 次数并读取 ITERATOR loopObject。
#[cfg(feature = "groovy")]
#[tokio::test]
async fn groovy_drives_for_and_iterator_nodes_with_loop_metadata() {
    liteflow_script_plugin::register_all().unwrap();
    let bus = FlowBus::new();
    bus.register(
        "groovy_tick",
        cmp(|ctx| async move {
            let count = ctx.get_data_as::<i64>("count").unwrap_or_default() + 1;
            ctx.set_data("count", json!(count));
            Ok(Value::Null)
        }),
    );
    bus.register_script_typed("groovy_for", "groovy", ScriptKind::For, "return 3")
        .unwrap();
    bus.register_script_typed(
        "groovy_values",
        "groovy",
        ScriptKind::Iterator,
        r#"return ["a", "b"]"#,
    )
    .unwrap();
    bus.register_script(
        "groovy_collect",
        "groovy",
        r#"
            def key = "joined"
            if (defaultContext.hasData(key)) {
                defaultContext.setData(key, defaultContext.getData(key) + "-" + _meta.loopObject)
            } else {
                defaultContext.setData(key, _meta.loopObject)
            }
        "#,
    )
    .unwrap();
    bus.add_chain(
        "groovy_loop_chain",
        "THEN(FOR(groovy_for).DO(groovy_tick), ITERATOR(groovy_values).DO(groovy_collect))",
    )
    .unwrap();

    let response = bus.execute("groovy_loop_chain").await;

    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(response.data("count"), Some(json!(3)));
    assert_eq!(response.data("joined"), Some(json!("a-b")));
}

/// 对应 Java Kotlin 验证与普通/布尔脚本基线：类型转换、val/var 和上下文写回。
#[cfg(feature = "kotlin")]
#[tokio::test]
async fn kotlin_executes_typed_baseline_and_rejects_compile_time_type_errors() {
    liteflow_script_plugin::register_all().unwrap();
    let bus = FlowBus::new();
    register_branch_nodes(&bus);
    bus.register_script(
        "kotlin_common",
        "kotlin",
        r#"
            val number: Int = "123".toInt()
            var score: Int = 2
            score = score + number
            defaultContext.setData("score", score)
        "#,
    )
    .unwrap();
    bus.register_script_typed(
        "kotlin_check",
        "kotlin",
        ScriptKind::Boolean,
        r#"
            val score: Int = defaultContext.getData("score")
            return score == 125
        "#,
    )
    .unwrap();
    bus.add_chain(
        "kotlin_chain",
        "THEN(kotlin_common, IF(kotlin_check, pass, fail))",
    )
    .unwrap();

    let response = bus.execute("kotlin_chain").await;

    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(response.data("score"), Some(json!(125)));
    assert_eq!(response.data("branch"), Some(json!("pass")));

    assert!(
        bus.register_script("kotlin_wrong_type", "kotlin", r#"val number: Int = "123""#,)
            .is_err()
    );
    assert!(
        bus.register_script(
            "kotlin_reassign_val",
            "kotlin",
            "val number: Int = 1\nnumber = 2",
        )
        .is_err()
    );
}
