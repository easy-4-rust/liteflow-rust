//! 脚本插件真实运行时验证。
//!
//! 每个测试都通过 `ScriptExecutorFactory -> FlowBus -> FlowExecutor` 完整链路执行，
//! 不直接调用插件内部实现，确保插件注册、节点构建、返回类型校验和 data 写回
//! 都处于验证范围内。

#[cfg(any(feature = "groovy", feature = "kotlin"))]
use std::any::Any;
#[cfg(any(feature = "groovy", feature = "kotlin", feature = "qlexpress"))]
use std::sync::Arc;
#[cfg(feature = "qlexpress")]
use std::sync::Mutex;
#[cfg(feature = "kotlin")]
use std::sync::RwLock;

use liteflow_core::FlowBus;
use liteflow_core::core::cmp;
#[cfg(any(feature = "groovy", feature = "kotlin", feature = "qlexpress"))]
use liteflow_core::script::ScriptBeanManager;
use liteflow_core::script::ScriptKind;
#[cfg(any(feature = "groovy", feature = "kotlin", feature = "qlexpress"))]
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
            qvm_sum = 0;
            for (i = 0; i < 3; i++) {
                qvm_sum += i;
            }
            defaultContext.setData("qvm_sum", qvm_sum);
            defaultContext.setData("score", a * b + 84);
            answer = ql_math.double(21);
            defaultContext.setData("answer", answer);
            order.setOrderType(a * b);
            order_type = order.getOrderType();
            defaultContext.setData("order_type", order_type);
            node_id = _meta.get("nodeId");
            defaultContext.setData("node_id", node_id);
            defaultContext.setData("curr_chain_id", _meta.get("currChainId"));
            defaultContext.setData("curr_chain_name", _meta.get("currChainName"));
            defaultContext.setData("request_copy", _meta.get("requestData"));
            defaultContext.setData("payload_value", payload.get("value"));
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
            json!({"trace_id": "ql-real-runtime"}),
            vec![
                ("order".to_string(), Arc::new(order_proxy)),
                ("order_state".to_string(), order_context),
                ("payload".to_string(), Arc::new(json!({"value": 7}))),
            ],
        )
        .await;

    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(response.data("score"), Some(json!(90)));
    assert_eq!(response.data("qvm_sum"), Some(json!(3)));
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
    assert_eq!(response.data("curr_chain_id"), Some(json!("ql_chain")));
    assert_eq!(response.data("curr_chain_name"), Some(json!("ql_chain")));
    assert_eq!(
        response.data("request_copy"),
        Some(json!({"trace_id": "ql-real-runtime"}))
    );
    assert_eq!(response.data("payload_value"), Some(json!(7)));
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

/// 对应 Java `common/flow.xml`：验证 Kotlin 表达式函数、块函数、bindings 上下文、
/// 普通/For/Boolean/Switch 五段真实 FlowBus 调用链。
#[cfg(feature = "kotlin")]
#[tokio::test]
async fn kotlin_executes_java_common_function_and_binding_baseline() {
    liteflow_script_plugin::register_all().unwrap();
    let bus = FlowBus::new();
    register_branch_nodes(&bus);
    bus.register(
        "kotlin_tick",
        cmp(|ctx| async move {
            let count = ctx.get_data_as::<i64>("kotlin_ticks").unwrap_or_default();
            ctx.set_data("kotlin_ticks", json!(count + 1));
            Ok(Value::Null)
        }),
    );
    bus.register_script(
        "kotlin_java_common",
        "kotlin",
        r#"
            import com.yomahub.liteflow.slot.DefaultContext

            fun sum(a: Int, b: Int) = a + b
            var a = 2
            var b = 3
            val defaultContext = bindings["defaultContext"] as DefaultContext
            defaultContext.setData("s1", sum(a, b))
            defaultContext.setData("k1", 1)
            defaultContext.setData("k2", 2)
            defaultContext.setData("count", 2)
            defaultContext.setData("route", "pass")
        "#,
    )
    .unwrap();
    bus.register_script_typed(
        "kotlin_java_for",
        "kotlin",
        ScriptKind::For,
        r#"
            fun getCount(): Int {
                val ctx = bindings["defaultContext"] as DefaultContext
                var n1 = ctx.getData("k1") as Int
                var n2 = ctx.getData("k2") as Int
                return n1 + n2
            }
            getCount()
        "#,
    )
    .unwrap();
    bus.register_script_typed(
        "kotlin_java_boolean",
        "kotlin",
        ScriptKind::Boolean,
        r#"
            fun getBoolean() = 2 > 1
            getBoolean()
        "#,
    )
    .unwrap();
    bus.register_script_typed(
        "kotlin_java_switch",
        "kotlin",
        ScriptKind::Switch,
        r#"
            fun getId(ctx: DefaultContext): String {
                return ctx.getData("route") as String
            }
            getId(bindings["defaultContext"] as DefaultContext)
        "#,
    )
    .unwrap();
    bus.register_script_typed(
        "kotlin_java_break",
        "kotlin",
        ScriptKind::Boolean,
        r#"
            fun isBreak(): Boolean {
                val ctx = bindings["defaultContext"] as DefaultContext
                var count = ctx.getData("count") as Int
                ctx.setData("count", --count)
                println("count: $count")
                return count < 0
            }
            isBreak()
        "#,
    )
    .unwrap();
    bus.add_chain(
        "kotlin_java_common_chain",
        "THEN(kotlin_java_common, FOR(kotlin_java_for).DO(kotlin_tick), IF(kotlin_java_boolean, pass, fail), SWITCH(kotlin_java_switch).TO(pass, fail), WHILE(kotlin_java_boolean).DO(kotlin_tick).BREAK(kotlin_java_break))",
    )
    .unwrap();

    let response = bus
        .execute_with_data("kotlin_java_common_chain", json!({"seed": true}))
        .await;

    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(response.data("s1"), Some(json!(5)));
    assert_eq!(response.data("kotlin_ticks"), Some(json!(6)));
    assert_eq!(response.data("count"), Some(json!(-1)));
    assert_eq!(response.data("branch"), Some(json!("pass")));
}

/// 对应 Java Kotlin `cmpdata/flow.xml`：验证 bindings 中的 `_meta` Map、cmpData
/// 结构化字段和 DefaultContext 别名写回。
#[cfg(feature = "kotlin")]
#[tokio::test]
async fn kotlin_reads_java_meta_and_structured_cmp_data_bindings() {
    liteflow_script_plugin::register_all().unwrap();
    let bus = FlowBus::new();
    bus.register_script(
        "kotlin_cmp_data",
        "kotlin",
        r#"
            import com.yomahub.liteflow.slot.DefaultContext

            var meta = bindings["_meta"] as Map<String, *>
            var cmpData = meta["cmpData"] as Map<String, *>
            var context = bindings["defaultContext"] as DefaultContext
            context.setData("birth", cmpData["birth"])
            context.setData("meta_node", meta["nodeId"])
            context.setData("meta_request", meta["requestData"])
        "#,
    )
    .unwrap();
    bus.add_chain(
        "kotlin_cmp_data_chain",
        r#"kotlin_cmp_data.data('{"name":"jack","birth":"1995-10-01"}')"#,
    )
    .unwrap();

    let response = bus
        .execute_with_data("kotlin_cmp_data_chain", json!({"request_id": 42}))
        .await;

    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(response.data("birth"), Some(json!("1995-10-01")));
    assert_eq!(response.data("meta_node"), Some(json!("kotlin_cmp_data")));
    assert_eq!(
        response.data("meta_request"),
        Some(json!({"request_id": 42}))
    );
}

/// 对应 Java Kotlin `scriptbean/flow.xml`：bindings 对象方法调用必须继续通过
/// ScriptBeanProxy 的 include/exclude 规则，而不是开放任意 Rust 反射。
#[cfg(feature = "kotlin")]
#[tokio::test]
async fn kotlin_invokes_controlled_script_bean_from_bindings() {
    liteflow_script_plugin::register_all().unwrap();
    ScriptBeanManager::add_script_bean(ScriptBeanProxy::new(
        "demo",
        &["getDemoStr2"],
        &[],
        [ScriptMethodProxy::new(
            "getDemoStr2",
            Arc::new(|arguments| {
                let name = arguments
                    .first()
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                Ok(json!(format!("hello,{name}")))
            }),
        )],
    ));
    let bus = FlowBus::new();
    bus.register_script(
        "kotlin_script_bean",
        "kotlin",
        r#"
            import com.yomahub.liteflow.slot.DefaultContext
            import com.example.DemoBean1

            var demoBean = bindings["demo"] as DemoBean1
            var greeting = demoBean.getDemoStr2("kobe")
            var context = bindings["defaultContext"] as DefaultContext
            context.setData("demo", greeting)
        "#,
    )
    .unwrap();
    bus.add_chain("kotlin_script_bean_chain", "kotlin_script_bean")
        .unwrap();

    let response = bus.execute("kotlin_script_bean_chain").await;

    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(response.data("demo"), Some(json!("hello,kobe")));
    ScriptBeanManager::remove_script_bean("demo");
}

/// 对应 Java Kotlin `scriptmethod/flow.xml`：同一个业务对象上的 `@ScriptMethod`
/// 必须按注解 value 分组为独立 bindings Bean，同时保留各自真实方法逻辑。
#[cfg(feature = "kotlin")]
#[tokio::test]
async fn kotlin_invokes_script_method_alias_groups_from_bindings() {
    liteflow_script_plugin::register_all().unwrap();
    ScriptBeanManager::add_script_bean(ScriptBeanProxy::new(
        "scriptMethodDemo",
        &[],
        &[],
        [ScriptMethodProxy::new(
            "getDemoStr1",
            Arc::new(|_| Ok(json!("hello"))),
        )],
    ));
    ScriptBeanManager::add_script_bean(ScriptBeanProxy::new(
        "scriptMethodDemo2",
        &[],
        &[],
        [ScriptMethodProxy::new(
            "getDemoStr2",
            Arc::new(|arguments| {
                let name = arguments
                    .first()
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                // 对应 Java DemoBean1#getDemoStr2 委托 DemoBean2 的真实返回语义。
                Ok(json!(format!("hello,{name}")))
            }),
        )],
    ));
    let bus = FlowBus::new();
    bus.register_script(
        "kotlin_script_method_one",
        "kotlin",
        r#"
            import com.yomahub.liteflow.slot.DefaultContext
            import com.example.DemoBean1

            var demo = bindings["scriptMethodDemo"] as DemoBean1
            var str = demo.getDemoStr1()
            var context = bindings["defaultContext"] as DefaultContext
            context.setData("script_method_one", str)
        "#,
    )
    .unwrap();
    bus.register_script(
        "kotlin_script_method_two",
        "kotlin",
        r#"
            import com.yomahub.liteflow.slot.DefaultContext
            import com.example.DemoBean1

            var demo2 = bindings["scriptMethodDemo2"] as DemoBean1
            var str = demo2.getDemoStr2("kobe")
            var context = bindings["defaultContext"] as DefaultContext
            context.setData("script_method_two", str)
        "#,
    )
    .unwrap();
    bus.add_chain(
        "kotlin_script_method_chain",
        "THEN(kotlin_script_method_one, kotlin_script_method_two)",
    )
    .unwrap();

    let response = bus.execute("kotlin_script_method_chain").await;

    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(response.data("script_method_one"), Some(json!("hello")));
    assert_eq!(
        response.data("script_method_two"),
        Some(json!("hello,kobe"))
    );
    ScriptBeanManager::remove_script_bean("scriptMethodDemo");
    ScriptBeanManager::remove_script_bean("scriptMethodDemo2");
}

/// 对应 Java Kotlin `throwException/flow.xml`：脚本抛出的 LiteFlowException
/// 必须穿过 Node 边界并保留业务错误码，不能退化为普通脚本文本错误。
#[cfg(feature = "kotlin")]
#[tokio::test]
async fn kotlin_liteflow_exception_preserves_business_code_in_response() {
    liteflow_script_plugin::register_all().unwrap();
    let bus = FlowBus::new();
    bus.register_script(
        "kotlin_throw",
        "kotlin",
        r#"
            import com.example.TestException
            throw TestException("T01", "测试错误")
        "#,
    )
    .unwrap();
    bus.add_chain("kotlin_throw_chain", "THEN(kotlin_throw)")
        .unwrap();

    let response = bus.execute("kotlin_throw_chain").await;

    assert!(!response.is_success());
    assert_eq!(response.get_code(), Some("T01"));
    assert!(response.get_message().contains("测试错误"));
}

/// 对应 Java Kotlin `refresh/flow.xml` 与 `flow_update.xml`：函数内 if/else
/// 必须参与真实 Switch 路由，刷新 XML 元数据后，既有 chain 应立即切换到新脚本
/// 并执行新增脚本节点。
#[cfg(feature = "kotlin")]
#[tokio::test]
async fn kotlin_refreshes_switch_control_flow_and_new_script_node() {
    liteflow_script_plugin::register_all().unwrap();
    let bus = FlowBus::new();
    register_branch_nodes(&bus);
    bus.register(
        "seed_count",
        cmp(|ctx| async move {
            ctx.set_data("count", json!(200));
            Ok(Value::Null)
        }),
    );

    let original_rule = r#"
        <flow>
            <nodes>
                <node id="kotlin_refresh_switch" name="选择脚本" type="switch_script" language="kotlin">
                    <![CDATA[
                        import com.yomahub.liteflow.slot.DefaultContext

                        fun getId(): String {
                            val context = bindings["defaultContext"] as DefaultContext
                            var count = context.getData("count") as Int
                            if(count > 100) {
                                return "pass"
                            } else {
                                return "fail"
                            }
                        }
                        getId()
                    ]]>
                </node>
            </nodes>
            <chain id="kotlin_refresh_chain">
                THEN(seed_count, SWITCH(kotlin_refresh_switch).TO(pass, fail));
            </chain>
        </flow>
    "#;
    bus.refresh_flow_meta_data(
        liteflow_core::enums::FlowParserTypeEnum::TypeElXml,
        original_rule,
    )
    .unwrap();

    let original_response = bus.execute("kotlin_refresh_chain").await;
    assert!(
        original_response.is_success(),
        "{:?}",
        original_response.cause
    );
    assert_eq!(original_response.data("branch"), Some(json!("pass")));
    assert_eq!(original_response.data("s2"), None);

    let updated_rule = r#"
        <flow>
            <nodes>
                <node id="kotlin_refresh_switch" name="选择脚本_改" type="switch_script" language="kotlin">
                    <![CDATA[
                        import com.yomahub.liteflow.slot.DefaultContext

                        fun getId(): String {
                            val context = bindings["defaultContext"] as DefaultContext
                            var count = context.getData("count") as Int
                            if(count > 100) {
                                return "fail"
                            } else {
                                return "pass"
                            }
                        }
                        getId()
                    ]]>
                </node>
                <node id="kotlin_refresh_s2" name="普通脚本_新增" type="script" language="kotlin">
                    <![CDATA[
                        import com.yomahub.liteflow.slot.DefaultContext

                        var a = 3
                        var b = 2
                        var c = 10
                        (bindings["defaultContext"] as? DefaultContext)?.setData("s2", a*b+c)
                    ]]>
                </node>
            </nodes>
            <chain id="kotlin_refresh_chain">
                THEN(seed_count, SWITCH(kotlin_refresh_switch).TO(pass, fail), kotlin_refresh_s2);
            </chain>
        </flow>
    "#;
    bus.refresh_flow_meta_data(
        liteflow_core::enums::FlowParserTypeEnum::TypeElXml,
        updated_rule,
    )
    .unwrap();

    let updated_response = bus.execute("kotlin_refresh_chain").await;
    assert!(
        updated_response.is_success(),
        "{:?}",
        updated_response.cause
    );
    assert_eq!(updated_response.data("branch"), Some(json!("fail")));
    assert_eq!(updated_response.data("s2"), Some(json!(16)));
    assert!(bus.contain_node("kotlin_refresh_s2"));
}

/// 对应 Java Kotlin `contextbean/flow.xml`：上下文 Bean 在 bindings 中优先于
/// 同名全局 ScriptBean，JavaBean getter/setter 必须读写 Slot 内同一个 serde
/// 对象，不能只修改脚本局部副本。
#[cfg(feature = "kotlin")]
#[tokio::test]
async fn kotlin_context_bean_getters_and_setters_preserve_object_identity() {
    liteflow_script_plugin::register_all().unwrap();
    let bus = FlowBus::new();
    bus.register_script(
        "kotlin_context_set",
        "kotlin",
        r#"
            import com.example.OrderContext
            import com.example.CheckContext
            import com.example.Order2Context

            var order = bindings["order"] as OrderContext
            var checkContext = bindings["checkContext"] as CheckContext
            var order2Context = bindings["order2Context"] as Order2Context

            order.setOrderNo("order1")
            checkContext.setSign("sign1")
            order2Context.setOrderNo("order2")
        "#,
    )
    .unwrap();
    bus.register_script(
        "kotlin_context_get",
        "kotlin",
        r#"
            import com.yomahub.liteflow.slot.DefaultContext
            import com.example.OrderContext
            import com.example.CheckContext
            import com.example.Order2Context

            var order = bindings["order"] as OrderContext
            var checkContext = bindings["checkContext"] as CheckContext
            var order2Context = bindings["order2Context"] as Order2Context
            var context = bindings["defaultContext"] as DefaultContext

            context.setData("read_order", order.getOrderNo())
            context.setData("read_sign", checkContext.getSign())
            context.setData("read_order2", order2Context.getOrderNo())
        "#,
    )
    .unwrap();
    bus.add_chain(
        "kotlin_context_chain",
        "THEN(kotlin_context_set, kotlin_context_get)",
    )
    .unwrap();

    let order = Arc::new(RwLock::new(json!({"orderNo": null, "orderType": 0})));
    let check_context = Arc::new(RwLock::new(json!({"sign": null, "randomId": 0})));
    let order2_context = Arc::new(RwLock::new(json!({"orderNo": null, "orderType": 0})));
    // Java bindParam 先放上下文 Bean，再以 putIfAbsent 放 ScriptBean；同名全局对象
    // 不得覆盖本次请求的 order。
    ScriptBeanManager::add_script_bean(ScriptBeanProxy::new(
        "order",
        &["getOrderNo", "setOrderNo"],
        &[],
        [
            ScriptMethodProxy::new("getOrderNo", Arc::new(|_| Ok(json!("global-order")))),
            ScriptMethodProxy::new("setOrderNo", Arc::new(|_| Ok(Value::Null))),
        ],
    ));
    let context_beans: Vec<(String, Arc<dyn Any + Send + Sync>)> = vec![
        ("order".to_string(), order.clone()),
        ("checkContext".to_string(), check_context.clone()),
        ("order2Context".to_string(), order2_context.clone()),
    ];

    let response = bus
        .execute_with("kotlin_context_chain", Value::Null, context_beans)
        .await;

    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(order.read().unwrap()["orderNo"], json!("order1"));
    assert_eq!(check_context.read().unwrap()["sign"], json!("sign1"));
    assert_eq!(order2_context.read().unwrap()["orderNo"], json!("order2"));
    assert_eq!(response.data("read_order"), Some(json!("order1")));
    assert_eq!(response.data("read_sign"), Some(json!("sign1")));
    assert_eq!(response.data("read_order2"), Some(json!("order2")));
    assert_eq!(
        response
            .get_context_bean::<RwLock<Value>>("order")
            .expect("响应应保留原始 order 上下文 Bean")
            .read()
            .unwrap()["orderNo"],
        json!("order1")
    );
    ScriptBeanManager::remove_script_bean("order");
}
