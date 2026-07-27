//! Vernal 生命周期、IoC 解析和 Web 适配的真实集成测试。

use std::sync::Arc;

use liteflow_core::core::cmp;
use liteflow_vernal::{
    LiteflowComponentRegistration, LiteflowConfig, LiteflowConfigGetter, LiteflowExecuteResponse,
    LiteflowParseMode, LiteflowRuleFormat, LiteflowRuntime, LiteflowVernalModule,
};
use serde_json::{Value, json};
use vernal_context::{ApplicationContext, VernalApplicationBuilder};

const INLINE_RULE: &str = r#"{
  "flow": {
    "chain": [
      {"id": "vernal_chain", "body": "vernal_component"}
    ]
  }
}"#;

const LAZY_JSON_RULE: &str = r#"{
  "flow": {
    "nodes": {
      "node": [
        {"id": "root_script", "type": "script", "value": "40 + 2"},
        {"id": "child_script", "type": "script", "value": "20 + 1"},
        {"id": "unused_script", "type": "script", "value": "1"}
      ]
    },
    "chain": [
      {"id": "root_chain", "body": "THEN(child_chain, root_script)"},
      {"id": "child_chain", "body": "THEN(child_script)"},
      {"id": "unused_chain", "body": "THEN(unused_script)"}
    ]
  }
}"#;

const LAZY_XML_RULE: &str = r#"<flow>
  <nodes>
    <node id="root_script" type="script">40 + 2</node>
    <node id="child_script" type="script">20 + 1</node>
    <node id="unused_script" type="script">1</node>
  </nodes>
  <chain id="root_chain"><body>THEN(child_chain, root_script)</body></chain>
  <chain id="child_chain"><body>THEN(child_script)</body></chain>
  <chain id="unused_chain"><body>THEN(unused_script)</body></chain>
</flow>"#;

const LAZY_YML_RULE: &str = r#"flow:
  nodes:
    node:
      - id: root_script
        type: script
        value: "40 + 2"
      - id: child_script
        type: script
        value: "20 + 1"
      - id: unused_script
        type: script
        value: "1"
  chain:
    - id: root_chain
      body: "THEN(child_chain, root_script)"
    - id: child_chain
      body: "THEN(child_script)"
    - id: unused_chain
      body: "THEN(unused_script)"
"#;

async fn ready_context() -> Arc<ApplicationContext> {
    let registration = LiteflowComponentRegistration::new("vernal_component", |flow_bus| {
        flow_bus.register(
            "vernal_component",
            cmp(|context| async move {
                let input = context
                    .inner
                    .input
                    .lock()
                    .ok()
                    .and_then(|input| input.get("value").cloned())
                    .unwrap_or(Value::Null);
                context.set_data("observed", input);
                Ok(Value::Null)
            }),
        );
        Ok(())
    });
    let module = LiteflowVernalModule::new(
        LiteflowConfig::new().with_inline_rule(LiteflowRuleFormat::Json, INLINE_RULE),
    )
    .with_component(registration);
    let mut builder = VernalApplicationBuilder::current().unwrap();
    builder.register_module(module).unwrap();
    builder.launch().await.unwrap()
}

/// 对应 Spring `@ConfigurationProperties`：验证 serde camelCase 和枚举值。
#[test]
fn liteflow_config_deserializes_spring_style_properties() {
    let config: LiteflowConfig = serde_json::from_value(json!({
        "enable": true,
        "inlineRule": INLINE_RULE,
        "ruleFormat": "json",
        "parseMode": "PARSE_ALL_ON_START",
        "chainCacheEnabled": true,
        "chainCacheCapacity": 11,
        "monitorEnableLog": true,
        "queueLimit": 17,
        "delay": 23,
        "period": 29,
        "globalThreadPoolExecutorClass": "test.GlobalExecutor",
        "globalThreadPoolSize": 7,
        "globalThreadPoolQueueSize": 19,
        "mainExecutorClass": "test.MainExecutor",
        "mainExecutorWorks": 3,
        "whenThreadPoolIsolate": true,
        "enableVirtualThread": false
    }))
    .unwrap();

    assert!(config.enable);
    assert_eq!(config.inline_rule.as_deref(), Some(INLINE_RULE));
    assert!(config.monitor_enable_log);
    assert!(config.is_chain_cache_enabled());
    assert_eq!(config.chain_cache_capacity(), 11);
    assert_eq!(config.queue_limit(), 17);
    assert_eq!(config.delay(), 23);
    assert_eq!(config.period(), 29);
    assert_eq!(
        config.global_thread_pool_executor_class(),
        "test.GlobalExecutor"
    );
    assert_eq!(config.global_thread_pool_size(), 7);
    assert_eq!(config.global_thread_pool_queue_size(), 19);
    assert_eq!(config.main_executor_class(), "test.MainExecutor");
    assert_eq!(config.main_executor_works(), 3);
    assert!(config.is_when_thread_pool_isolate());
    assert!(!config.is_enable_virtual_thread());
    let core_config = config.to_core_config();
    assert!(core_config.is_chain_cache_enabled());
    assert_eq!(core_config.chain_cache_capacity(), 11);
}

/// 验证 Java MonitorBus 构造语义：配置容量、自动启动以及容器关闭时停止调度器。
#[tokio::test]
async fn vernal_lifecycle_starts_and_stops_configured_monitor_task() {
    let config = LiteflowConfig {
        monitor_enable_log: true,
        queue_limit: 3,
        delay: 1,
        period: 2,
        ..LiteflowConfig::default()
    };
    let module = LiteflowVernalModule::new(config);
    let mut builder = VernalApplicationBuilder::current().unwrap();
    builder.register_module(module).unwrap();
    let context = builder.launch().await.unwrap();
    let runtime: Arc<LiteflowRuntime> = context.container().resolve().unwrap();

    assert_eq!(runtime.flow_bus().monitor().queue_limit(), 3);
    assert!(runtime.is_monitor_task_running());
    tokio::time::sleep(std::time::Duration::from_millis(8)).await;
    assert!(runtime.is_monitor_task_running());

    context.close().await.unwrap();
    assert!(!runtime.is_monitor_task_running());
}

/// 验证 Java LiteflowConfigGetter 的 set/get/clean 回退契约。
#[test]
fn liteflow_config_getter_preserves_compatibility_contract() {
    let configured = LiteflowConfig {
        print_execution_log: false,
        monitor_enable_log: true,
        ..LiteflowConfig::default()
    };
    LiteflowConfigGetter::set_liteflow_config(configured.to_core_config());
    assert_eq!(LiteflowConfigGetter::get(), configured.to_core_config());

    LiteflowConfigGetter::clean();
    assert_eq!(
        LiteflowConfigGetter::get(),
        LiteflowConfig::default().to_core_config()
    );
}

/// 验证三种 Java ParseMode 枚举值均可从 Spring 风格配置反序列化。
#[test]
fn liteflow_config_deserializes_all_java_parse_modes() {
    for (source, expected) in [
        ("PARSE_ALL_ON_START", LiteflowParseMode::ParseAllOnStart),
        (
            "PARSE_ALL_ON_FIRST_EXEC",
            LiteflowParseMode::ParseAllOnFirstExec,
        ),
        (
            "PARSE_ONE_ON_FIRST_EXEC",
            LiteflowParseMode::ParseOneOnFirstExec,
        ),
    ] {
        let config: LiteflowConfig = serde_json::from_value(json!({"parseMode": source})).unwrap();
        assert_eq!(config.parse_mode, expected);
    }
}

/// 验证首次执行全解析：执行前无 Chain，并发首次请求后全部 Chain 与脚本均已物化。
#[tokio::test]
async fn parse_all_on_first_exec_is_concurrency_safe_and_builds_every_rule() {
    let runtime = Arc::new(LiteflowRuntime::new(
        liteflow_core::FlowBus::new(),
        LiteflowConfig {
            inline_rule: Some(LAZY_JSON_RULE.to_string()),
            parse_mode: LiteflowParseMode::ParseAllOnFirstExec,
            ..LiteflowConfig::default()
        },
    ));
    assert!(runtime.flow_bus().chain_ids().is_empty());

    let mut tasks = Vec::new();
    for _ in 0..8 {
        let runtime = Arc::clone(&runtime);
        tasks.push(tokio::spawn(async move {
            runtime
                .try_execute("root_chain", Value::Null)
                .await
                .unwrap()
        }));
    }
    for task in tasks {
        let response = task.await.unwrap();
        assert!(response.is_success(), "{:?}", response.cause);
    }

    for chain_id in ["root_chain", "child_chain", "unused_chain"] {
        assert!(runtime.flow_bus().contains_chain(chain_id));
    }
    for node_id in ["root_script", "child_script", "unused_script"] {
        assert!(runtime.flow_bus().contains_node(node_id));
    }
}

/// 验证首次执行单链解析在 JSON/XML/YAML 中只物化目标依赖闭包。
#[tokio::test]
async fn parse_one_on_first_exec_builds_only_target_dependency_closure() {
    for (format, source) in [
        (LiteflowRuleFormat::Json, LAZY_JSON_RULE),
        (LiteflowRuleFormat::Xml, LAZY_XML_RULE),
        (LiteflowRuleFormat::Yml, LAZY_YML_RULE),
    ] {
        let runtime = LiteflowRuntime::new(
            liteflow_core::FlowBus::new(),
            LiteflowConfig {
                inline_rule: Some(source.to_string()),
                rule_format: format,
                parse_mode: LiteflowParseMode::ParseOneOnFirstExec,
                ..LiteflowConfig::default()
            },
        );
        assert!(runtime.flow_bus().chain_ids().is_empty());
        assert!(!runtime.flow_bus().contains_node("root_script"));

        let response = runtime
            .try_execute("root_chain", Value::Null)
            .await
            .unwrap();
        assert!(response.is_success(), "{:?}", response.cause);
        assert!(runtime.flow_bus().contains_chain("root_chain"));
        assert!(runtime.flow_bus().contains_chain("child_chain"));
        assert!(runtime.flow_bus().contains_node("root_script"));
        assert!(runtime.flow_bus().contains_node("child_script"));
        assert!(!runtime.flow_bus().contains_chain("unused_chain"));
        assert!(!runtime.flow_bus().contains_node("unused_script"));
    }
}

/// 验证 Chain 缓存淘汰后由既有规则计划重新物化，而不是永久丢失 Chain。
///
/// 对应 Java `ChainCacheLifeCycle` + `PARSE_ONE_ON_FIRST_EXEC` 的完整执行闭环。
#[tokio::test]
async fn parse_one_chain_cache_evicts_and_rebuilds_compiled_chain() {
    let runtime = LiteflowRuntime::new(
        liteflow_core::FlowBus::new(),
        LiteflowConfig {
            inline_rule: Some(LAZY_JSON_RULE.to_string()),
            rule_format: LiteflowRuleFormat::Json,
            parse_mode: LiteflowParseMode::ParseOneOnFirstExec,
            chain_cache_enabled: true,
            chain_cache_capacity: 1,
            ..LiteflowConfig::default()
        },
    );

    let root = runtime
        .try_execute("root_chain", Value::Null)
        .await
        .unwrap();
    assert!(root.is_success(), "{:?}", root.cause);
    assert!(runtime.flow_bus().contains_chain("root_chain"));

    let unused = runtime
        .try_execute("unused_chain", Value::Null)
        .await
        .unwrap();
    assert!(unused.is_success(), "{:?}", unused.cause);
    assert!(
        !runtime.flow_bus().contains_chain("root_chain"),
        "容量为 1 时执行第二条顶层链必须淘汰第一条已编译链"
    );
    assert!(runtime.flow_bus().contains_chain("unused_chain"));

    let rebuilt = runtime
        .try_execute("root_chain", Value::Null)
        .await
        .unwrap();
    assert!(rebuilt.is_success(), "{:?}", rebuilt.cause);
    assert!(runtime.flow_bus().contains_chain("root_chain"));
    assert!(
        !runtime.flow_bus().contains_chain("unused_chain"),
        "重新执行被淘汰链时应由规则计划重建，并淘汰上一条链"
    );
}

/// 验证 Java `FlowExecutor#initChainCache` 的容量校验不会被 Rust 默认值掩盖。
#[tokio::test]
async fn parse_one_chain_cache_rejects_zero_capacity() {
    let runtime = LiteflowRuntime::new(
        liteflow_core::FlowBus::new(),
        LiteflowConfig {
            inline_rule: Some(LAZY_JSON_RULE.to_string()),
            parse_mode: LiteflowParseMode::ParseOneOnFirstExec,
            chain_cache_enabled: true,
            chain_cache_capacity: 0,
            ..LiteflowConfig::default()
        },
    );

    let error = runtime
        .try_execute("root_chain", Value::Null)
        .await
        .expect_err("容量为 0 必须在规则执行前拒绝初始化");
    assert!(error.to_string().contains("greater than 0"));
    assert!(runtime.flow_bus().chain_ids().is_empty());
}

/// 验证文件规则源同样经过按链延迟计划，而不是回退为启动期全量构建。
#[tokio::test]
async fn parse_one_on_first_exec_supports_file_rule_source() {
    let file = std::env::temp_dir().join(format!(
        "liteflow-parse-one-{}-{}.json",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    std::fs::write(&file, LAZY_JSON_RULE).unwrap();
    let runtime = LiteflowRuntime::new(
        liteflow_core::FlowBus::new(),
        LiteflowConfig {
            rule_source: Some(file.to_string_lossy().into_owned()),
            parse_mode: LiteflowParseMode::ParseOneOnFirstExec,
            ..LiteflowConfig::default()
        },
    );

    let response = runtime
        .try_execute("child_chain", Value::Null)
        .await
        .unwrap();
    let _ = std::fs::remove_file(file);

    assert!(response.is_success(), "{:?}", response.cause);
    assert!(runtime.flow_bus().contains_chain("child_chain"));
    assert!(runtime.flow_bus().contains_node("child_script"));
    assert!(!runtime.flow_bus().contains_chain("root_chain"));
    assert!(!runtime.flow_bus().contains_node("root_script"));
    assert!(!runtime.flow_bus().contains_node("unused_script"));
}

/// 验证 Vernal 启动阶段区分两种延迟模式：全量模式不读规则，单链模式只收集定义。
#[tokio::test]
async fn vernal_lifecycle_preserves_both_lazy_parse_mode_boundaries() {
    for parse_mode in [
        LiteflowParseMode::ParseAllOnFirstExec,
        LiteflowParseMode::ParseOneOnFirstExec,
    ] {
        let module = LiteflowVernalModule::new(LiteflowConfig {
            inline_rule: Some(LAZY_JSON_RULE.to_string()),
            parse_mode,
            ..LiteflowConfig::default()
        });
        let mut builder = VernalApplicationBuilder::current().unwrap();
        builder.register_module(module).unwrap();
        let context = builder.launch().await.unwrap();
        let runtime: Arc<LiteflowRuntime> = context.container().resolve().unwrap();

        assert!(runtime.flow_bus().chain_ids().is_empty());
        assert!(!runtime.flow_bus().contains_node("root_script"));
        let response = runtime
            .try_execute("root_chain", Value::Null)
            .await
            .unwrap();
        assert!(response.is_success(), "{:?}", response.cause);
        assert!(runtime.flow_bus().contains_chain("root_chain"));
        assert!(runtime.flow_bus().contains_chain("child_chain"));
        assert_eq!(
            runtime.flow_bus().contains_chain("unused_chain"),
            parse_mode == LiteflowParseMode::ParseAllOnFirstExec
        );
        assert_eq!(
            runtime.flow_bus().contains_node("unused_script"),
            parse_mode == LiteflowParseMode::ParseAllOnFirstExec
        );
        context.close().await.unwrap();
    }
}

/// 验证 ApplicationModule 原子装配、Lifecycle 规则初始化和同一容器解析。
#[tokio::test]
async fn vernal_module_launches_and_executes_registered_component() {
    let context = ready_context().await;
    let runtime: Arc<LiteflowRuntime> = context.container().resolve().unwrap();
    let flow_bus: Arc<liteflow_core::FlowBus> = context.container().resolve().unwrap();

    let response = runtime.execute("vernal_chain", json!({"value": 42})).await;

    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(response.data("observed"), Some(json!(42)));
    assert!(flow_bus.contains_node("vernal_component"));
    context.close().await.unwrap();
}

/// 验证真实 Axum Router、Vernal 请求作用域和 JSON 协议边界。
#[cfg(feature = "axum")]
#[tokio::test]
async fn axum_endpoint_resolves_runtime_from_vernal_context() {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use liteflow_vernal::LiteflowAxumRouter;
    use tower::ServiceExt;

    let context = ready_context().await;
    let app = LiteflowAxumRouter::with_context(Arc::clone(&context));
    let request = Request::builder()
        .method("POST")
        .uri("/liteflow/execute/vernal_chain")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"data": {"value": 7}, "requestId": "http-rid"}).to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: LiteflowExecuteResponse = serde_json::from_slice(&bytes).unwrap();
    assert!(body.success);
    assert_eq!(body.request_id, "http-rid");
    assert!(body.steps.contains("vernal_component"));
    context.close().await.unwrap();
}

/// 验证真实 Actix App、Vernal 中间件和组件提取器。
#[cfg(feature = "actix")]
#[actix_web::test]
async fn actix_endpoint_resolves_runtime_from_vernal_context() {
    use actix_web::{App, http::StatusCode, test};
    use liteflow_vernal::LiteflowActixService;

    let context = ready_context().await;
    let service = test::init_service(App::new().configure({
        let context = Arc::clone(&context);
        move |config| LiteflowActixService::configure(config, context)
    }))
    .await;
    let request = test::TestRequest::post()
        .uri("/liteflow/execute/vernal_chain")
        .set_json(json!({"data": {"value": 9}, "requestId": "actix-rid"}))
        .to_request();

    let response = test::call_service(&service, request).await;

    assert_eq!(response.status(), StatusCode::OK);
    let body: LiteflowExecuteResponse = test::read_body_json(response).await;
    assert!(body.success);
    assert_eq!(body.request_id, "actix-rid");
    context.close().await.unwrap();
}
