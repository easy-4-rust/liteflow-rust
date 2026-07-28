//! Kotlin 脚本与 Vernal 组合场景。
//!
//! 对应 Java:
//! `liteflow-testcase-el-script-kotlin-springboot`。

use std::any::Any;
use std::sync::{Arc, RwLock};

use liteflow_core::script::proxy::{ScriptBeanProxy, ScriptMethodProxy};
use liteflow_core::script::{ScriptBeanManager, ScriptExecutorFactory, ScriptKind};
use liteflow_core::{FlowBus, FlowParserTypeEnum};
use liteflow_script_kotlin::KotlinScriptExecutor;
use liteflow_vernal::{LiteflowComponentRegistration, LiteflowConfig, VernalComponentScanner};

/// 注册并真实执行 Java Kotlin testcase 的函数、bindings 与强类型返回基线。
///
/// 返回执行器注册、Vernal 配置、普通脚本、For 脚本和上下文写回是否全部成功。
/// 对应 Java: `LiteFlowKotlinScriptCommonELTest#testCommonScript1` 与
/// `LiteFlowKotlinScriptCommonELTest#testForScript1`。
pub async fn run_case() -> bool {
    if KotlinScriptExecutor::register().is_err()
        || !ScriptExecutorFactory::contains("kotlin")
        || !LiteflowConfig::new().enable
    {
        return false;
    }

    let bus = FlowBus::new();
    let common_registered = bus.register_script(
        "kotlin_case",
        "kotlin",
        r#"
            import com.yomahub.liteflow.slot.DefaultContext

            fun sum(a: Int, b: Int) = a + b
            var a = 2
            var b = 3
            val context = bindings["defaultContext"] as DefaultContext
            context.setData("kotlin", sum(a, b))
            context.setData("k1", 1)
            context.setData("k2", 2)
        "#,
    );
    let for_registered = bus.register_script_typed(
        "kotlin_count",
        "kotlin",
        ScriptKind::For,
        r#"
            fun getCount(): Int {
                val context = bindings["defaultContext"] as DefaultContext
                var left = context.getData("k1") as Int
                var right = context.getData("k2") as Int
                return left + right
            }
            getCount()
        "#,
    );
    // Java testcase 的 a 组件在循环中执行三次；Rust 用同一真实节点计数验证。
    bus.register(
        "kotlin_tick",
        liteflow_core::core::cmp(|context| async move {
            let count = context
                .get_data_as::<i64>("kotlin_ticks")
                .unwrap_or_default();
            context.set_data("kotlin_ticks", serde_json::json!(count + 1));
            Ok(serde_json::Value::Null)
        }),
    );
    let chain_registered = bus.add_chain(
        "kotlin_case_chain",
        "THEN(kotlin_case, FOR(kotlin_count).DO(kotlin_tick))",
    );
    let throw_registered = bus.register_script(
        "kotlin_throw",
        "kotlin",
        r#"
            import com.example.TestException
            throw TestException("T01", "测试错误")
        "#,
    );
    let throw_chain_registered = bus.add_chain("kotlin_throw_chain", "THEN(kotlin_throw)");
    if common_registered.is_err()
        || for_registered.is_err()
        || chain_registered.is_err()
        || throw_registered.is_err()
        || throw_chain_registered.is_err()
    {
        return false;
    }

    let response = bus.execute("kotlin_case_chain").await;
    let throw_response = bus.execute("kotlin_throw_chain").await;
    response.is_success()
        && response.data_as::<i64>("kotlin") == Some(5)
        && response.data_as::<i64>("kotlin_ticks") == Some(3)
        && !throw_response.is_success()
        && throw_response.get_code() == Some("T01")
        && throw_response.get_message().contains("测试错误")
        && run_refresh_case(&bus).await
        && run_context_bean_case(&bus).await
        && run_script_method_case(&bus).await
}

/// 通过 Vernal 扫描链执行 Java `@ScriptMethod` 分组后的 Kotlin testcase。
///
/// Java 的 `demo` 与 `demo2` 来自同一 `DemoBean1` 上两个注解 value；Rust 在
/// 显式注册期完成等价分组，再由 `ScriptMethodBeanProcess` 写入真实脚本注册表。
async fn run_script_method_case(bus: &FlowBus) -> bool {
    let demo = ScriptBeanProxy::new(
        "demo",
        &[],
        &[],
        [ScriptMethodProxy::new(
            "getDemoStr1",
            Arc::new(|_| Ok(serde_json::json!("hello"))),
        )],
    );
    let demo2 = ScriptBeanProxy::new(
        "demo2",
        &[],
        &[],
        [ScriptMethodProxy::new(
            "getDemoStr2",
            Arc::new(|arguments| {
                let name = arguments
                    .first()
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                // 对应 Java DemoBean1#getDemoStr2 对 DemoBean2 的依赖调用结果。
                Ok(serde_json::json!(format!("hello,{name}")))
            }),
        )],
    );
    let scanner = VernalComponentScanner::with_config(
        &LiteflowConfig {
            print_banner: false,
            ..LiteflowConfig::default()
        },
        vec![LiteflowComponentRegistration::script_methods(
            "demoBean1",
            vec![demo, demo2],
        )],
    );
    if scanner.scan(bus).is_err()
        || scanner.scanned_component_ids() != vec!["demoBean1".to_string()]
    {
        return false;
    }

    let registered = bus.register_script(
        "script_method_case",
        "kotlin",
        r#"
            import com.yomahub.liteflow.slot.DefaultContext
            import com.example.DemoBean1

            var demo = bindings["demo"] as DemoBean1
            var demo2 = bindings["demo2"] as DemoBean1
            var context = bindings["defaultContext"] as DefaultContext
            context.setData("script_method_one", demo.getDemoStr1())
            context.setData("script_method_two", demo2.getDemoStr2("kobe"))
        "#,
    );
    let chained = bus.add_chain("script_method_chain", "THEN(script_method_case)");
    if registered.is_err() || chained.is_err() {
        ScriptBeanManager::remove_script_bean("demo");
        ScriptBeanManager::remove_script_bean("demo2");
        return false;
    }

    let response = bus.execute("script_method_chain").await;
    ScriptBeanManager::remove_script_bean("demo");
    ScriptBeanManager::remove_script_bean("demo2");
    response.is_success()
        && response.data("script_method_one") == Some(serde_json::json!("hello"))
        && response.data("script_method_two") == Some(serde_json::json!("hello,kobe"))
}

/// 通过 Vernal 组合入口执行 Java contextbean testcase 的同对象读写。
async fn run_context_bean_case(bus: &FlowBus) -> bool {
    if bus
        .register_script(
            "context_bean_case",
            "kotlin",
            r#"
                var order = bindings["order"] as OrderContext
                var checkContext = bindings["checkContext"] as CheckContext
                var order2Context = bindings["order2Context"] as Order2Context
                var context = bindings["defaultContext"] as DefaultContext

                order.setOrderNo("order1")
                checkContext.setSign("sign1")
                order2Context.setOrderNo("order2")
                context.setData("context_order", order.getOrderNo())
                context.setData("context_sign", checkContext.getSign())
                context.setData("context_order2", order2Context.getOrderNo())
            "#,
        )
        .is_err()
        || bus
            .add_chain("context_bean_chain", "THEN(context_bean_case)")
            .is_err()
    {
        return false;
    }

    let order = Arc::new(RwLock::new(serde_json::json!({"orderNo": null})));
    let check_context = Arc::new(RwLock::new(serde_json::json!({"sign": null})));
    let order2_context = Arc::new(RwLock::new(serde_json::json!({"orderNo": null})));
    let context_beans: Vec<(String, Arc<dyn Any + Send + Sync>)> = vec![
        ("order".to_string(), order.clone()),
        ("checkContext".to_string(), check_context.clone()),
        ("order2Context".to_string(), order2_context.clone()),
    ];
    let response = bus
        .execute_with("context_bean_chain", serde_json::Value::Null, context_beans)
        .await;
    response.is_success()
        && order
            .read()
            .is_ok_and(|value| value["orderNo"] == serde_json::json!("order1"))
        && check_context
            .read()
            .is_ok_and(|value| value["sign"] == serde_json::json!("sign1"))
        && order2_context
            .read()
            .is_ok_and(|value| value["orderNo"] == serde_json::json!("order2"))
        && response.data("context_order") == Some(serde_json::json!("order1"))
        && response.data("context_sign") == Some(serde_json::json!("sign1"))
        && response.data("context_order2") == Some(serde_json::json!("order2"))
}

/// 通过 Vernal 组合入口执行 Java refresh testcase 的旧、新两版 XML。
async fn run_refresh_case(bus: &FlowBus) -> bool {
    for (node_id, branch) in [("refresh_pass", "pass"), ("refresh_fail", "fail")] {
        let branch = branch.to_string();
        bus.register(
            node_id,
            liteflow_core::core::cmp(move |context| {
                let branch = branch.clone();
                async move {
                    context.set_data("refresh_branch", serde_json::json!(branch));
                    Ok(serde_json::Value::Null)
                }
            }),
        );
    }
    bus.register(
        "refresh_seed",
        liteflow_core::core::cmp(|context| async move {
            context.set_data("count", serde_json::json!(200));
            Ok(serde_json::Value::Null)
        }),
    );

    let original = r#"
        <flow>
          <nodes>
            <node id="refresh_switch" type="switch_script" language="kotlin"><![CDATA[
              fun getId(): String {
                val context = bindings["defaultContext"] as DefaultContext
                var count = context.getData("count") as Int
                if(count > 100) {
                  return "refresh_pass"
                } else {
                  return "refresh_fail"
                }
              }
              getId()
            ]]></node>
          </nodes>
          <chain id="refresh_chain">
            THEN(refresh_seed, SWITCH(refresh_switch).TO(refresh_pass, refresh_fail));
          </chain>
        </flow>
    "#;
    if bus
        .refresh_flow_meta_data(FlowParserTypeEnum::TypeElXml, original)
        .is_err()
    {
        return false;
    }
    let original_response = bus.execute("refresh_chain").await;
    if !original_response.is_success()
        || original_response.data("refresh_branch") != Some(serde_json::json!("pass"))
    {
        return false;
    }

    let updated = r#"
        <flow>
          <nodes>
            <node id="refresh_switch" type="switch_script" language="kotlin"><![CDATA[
              fun getId(): String {
                val context = bindings["defaultContext"] as DefaultContext
                var count = context.getData("count") as Int
                if(count > 100) {
                  return "refresh_fail"
                } else {
                  return "refresh_pass"
                }
              }
              getId()
            ]]></node>
            <node id="refresh_added" type="script" language="kotlin"><![CDATA[
              (bindings["defaultContext"] as? DefaultContext)?.setData("refresh_added", true)
            ]]></node>
          </nodes>
          <chain id="refresh_chain">
            THEN(refresh_seed, SWITCH(refresh_switch).TO(refresh_pass, refresh_fail), refresh_added);
          </chain>
        </flow>
    "#;
    if bus
        .refresh_flow_meta_data(FlowParserTypeEnum::TypeElXml, updated)
        .is_err()
    {
        return false;
    }
    let updated_response = bus.execute("refresh_chain").await;
    updated_response.is_success()
        && updated_response.data("refresh_branch") == Some(serde_json::json!("fail"))
        && updated_response.data("refresh_added") == Some(serde_json::json!(true))
}
