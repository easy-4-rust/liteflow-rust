//! Vernal 生命周期、IoC 解析和 Web 适配的真实集成测试。

use std::sync::Arc;

use liteflow_core::core::cmp;
use liteflow_vernal::{
    LiteflowComponentRegistration, LiteflowConfig, LiteflowConfigGetter, LiteflowExecuteResponse,
    LiteflowRuleFormat, LiteflowRuntime, LiteflowVernalModule,
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
        "monitorEnableLog": true
    }))
    .unwrap();

    assert!(config.enable);
    assert_eq!(config.inline_rule.as_deref(), Some(INLINE_RULE));
    assert!(config.monitor_enable_log);
}

/// 验证 Java LiteflowConfigGetter 的 set/get/clean 回退契约。
#[test]
fn liteflow_config_getter_preserves_compatibility_contract() {
    let configured = LiteflowConfig {
        print_execution_log: false,
        monitor_enable_log: true,
        ..LiteflowConfig::default()
    };
    LiteflowConfigGetter::set_liteflow_config(configured.clone());
    assert_eq!(LiteflowConfigGetter::get(), configured);

    LiteflowConfigGetter::clean();
    assert_eq!(LiteflowConfigGetter::get(), LiteflowConfig::default());
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
