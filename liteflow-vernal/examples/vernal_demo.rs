//! 可启动的 Vernal + Axum LiteFlow 示例。
//!
//! 启动后可执行：
//! `curl -X POST http://127.0.0.1:3000/liteflow/execute/demo -H 'content-type: application/json' -d '{"data":{"name":"LiteFlow"}}'`

use std::sync::Arc;

use liteflow_core::cmp;
use liteflow_vernal::{
    LiteflowAxumRouter, LiteflowComponentRegistration, LiteflowConfig, LiteflowRuleFormat,
    LiteflowVernalModule,
};
use serde_json::Value;
use vernal_context::VernalApplicationBuilder;

const RULE: &str = r#"{
  "flow": {
    "chain": [
      {"id": "demo", "body": "hello"}
    ]
  }
}"#;

#[tokio::main]
async fn main() {
    let component = LiteflowComponentRegistration::new("hello", |flow_bus| {
        flow_bus.register(
            "hello",
            cmp(|context| async move {
                context.set_data("message", Value::String("hello from Vernal".to_string()));
                Ok(Value::Null)
            }),
        );
        Ok(())
    });
    let module = LiteflowVernalModule::new(
        LiteflowConfig::new().with_inline_rule(LiteflowRuleFormat::Json, RULE),
    )
    .with_component(component);
    let mut builder = VernalApplicationBuilder::current().expect("Tokio runtime should exist");
    builder
        .register_module(module)
        .expect("LiteFlow module should be valid");
    let context = builder
        .launch()
        .await
        .expect("Vernal context should launch");
    let router = LiteflowAxumRouter::with_context(Arc::clone(&context));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("demo port should be available");

    println!("LiteFlow Vernal demo listening on http://127.0.0.1:3000");
    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Ctrl-C handler should install");
            context.close().await.expect("context should close");
        })
        .await
        .expect("Axum server should run");
}
