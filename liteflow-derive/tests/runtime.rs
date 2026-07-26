use std::any::Any;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use liteflow_core::{CmpContext, FlowBus, LiteflowError, NodeComponent};
use liteflow_derive::{
    alias_for, context_bean, fallback_cmp, liteflow_cmp_define, liteflow_component, liteflow_retry,
};
use serde_json::{Value, json};

/// 普通注解组件。对应 Java `@LiteflowComponent("record")`。
#[liteflow_component("record", name = "记录组件")]
struct RecordCmp;

#[async_trait]
impl NodeComponent for RecordCmp {
    async fn process(&self, ctx: &CmpContext) -> Result<Value, LiteflowError> {
        ctx.set_data("record_ran", json!(true));
        Ok(Value::Null)
    }

    fn name(&self) -> &str {
        "原始名称"
    }
}

/// 声明式组件。对应 Java `@LiteflowCmpDefine` + 多个 `@LiteflowMethod`。
struct OrderCmp;

struct StockFact {
    available: bool,
}

/// Java `@ContextBean("stockFact")` 的 Rust 别名元数据。
#[context_bean("stockFact")]
struct AnnotatedStockFact {
    available: bool,
}

#[liteflow_cmp_define("orderCmp")]
impl OrderCmp {
    #[liteflow_method("checkStock")]
    async fn check_stock(
        &self,
        ctx: &CmpContext,
        #[liteflow_fact("stockFact")] stock: Arc<StockFact>,
    ) -> Result<Value, LiteflowError> {
        ctx.set_data("stock_checked", json!(stock.available));
        Ok(Value::Null)
    }

    #[liteflow_method("isVip")]
    async fn is_vip(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(json!(true))
    }
}

struct RetryCmp {
    attempts: Arc<AtomicUsize>,
}

#[liteflow_retry(2, for = ["java.text.ParseException"])]
#[async_trait]
impl NodeComponent for RetryCmp {
    async fn process(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        let attempt = self.attempts.fetch_add(1, Ordering::SeqCst);
        if attempt < 2 {
            return Err(LiteflowError::Parse("temporary".to_string()));
        }
        Ok(Value::Null)
    }
}

/// Java `@FallbackCmp` 的 Rust 编译期元数据与真实 BOOLEAN 降级组件。
#[fallback_cmp("booleanFallback", node_type = "boolean")]
struct BooleanFallbackCmp;

#[async_trait]
impl NodeComponent for BooleanFallbackCmp {
    async fn process(&self, ctx: &CmpContext) -> Result<Value, LiteflowError> {
        ctx.set_data("fallback_expected_id", json!(ctx.node_id()));
        Ok(json!(true))
    }
}

#[alias_for(annotation = "liteflow_method", attribute = "node_id")]
fn alias_target() -> usize {
    7
}

#[tokio::test]
async fn component_macro_registers_and_executes_real_chain() {
    let bus = FlowBus::new();
    RecordCmp.register(&bus).unwrap();
    bus.add_chain("component_chain", "THEN(record)").unwrap();

    let response = bus.execute("component_chain").await;

    assert!(response.is_success(), "{}", response.message);
    assert_eq!(response.data("record_ran"), Some(json!(true)));
    assert_eq!(response.steps[0].node_name, "记录组件");
    assert_eq!(RecordCmp::LITEFLOW_NODE_ID, "record");
    assert_eq!(RecordCmp::LITEFLOW_NODE_NAME, "记录组件");
}

#[tokio::test]
async fn declarative_macro_dispatches_multiple_methods_through_el() {
    let bus = FlowBus::new();
    OrderCmp.register_decl(&bus);
    bus.register(
        "vip",
        liteflow_core::cmp(|ctx| async move {
            ctx.set_data("plan", json!("vip"));
            Ok(Value::Null)
        }),
    );
    bus.register(
        "normal",
        liteflow_core::cmp(|ctx| async move {
            ctx.set_data("plan", json!("normal"));
            Ok(Value::Null)
        }),
    );
    bus.add_chain(
        "decl_chain",
        "THEN(orderCmp.checkStock, IF(orderCmp.isVip, vip, normal))",
    )
    .unwrap();

    let beans: Vec<(String, Arc<dyn Any + Send + Sync>)> = vec![(
        "stockFact".to_string(),
        Arc::new(StockFact { available: true }),
    )];
    let response = bus.execute_with("decl_chain", Value::Null, beans).await;

    assert!(response.is_success(), "{}", response.message);
    assert_eq!(response.data("stock_checked"), Some(json!(true)));
    assert_eq!(response.data("plan"), Some(json!("vip")));
    assert_eq!(OrderCmp::LITEFLOW_DECL_ID, "orderCmp");
}

#[tokio::test]
async fn context_bean_macro_registers_alias_for_real_execution() {
    let bus = FlowBus::new();
    bus.register(
        "read_context",
        liteflow_core::cmp(|context| async move {
            let stock = context
                .bean::<AnnotatedStockFact>(AnnotatedStockFact::LITEFLOW_CONTEXT_NAME)
                .expect("annotated context bean should be available");
            context.set_data("annotated_available", json!(stock.available));
            Ok(Value::Null)
        }),
    );
    bus.add_chain("context_chain", "THEN(read_context)")
        .unwrap();

    let response = bus
        .execute_with(
            "context_chain",
            Value::Null,
            vec![AnnotatedStockFact { available: true }.into_context_bean()],
        )
        .await;

    assert!(response.is_success(), "{}", response.message);
    assert_eq!(response.data("annotated_available"), Some(json!(true)));
    assert_eq!(AnnotatedStockFact::LITEFLOW_CONTEXT_NAME, "stockFact");
}

#[tokio::test]
async fn retry_macro_applies_count_and_exception_filter_at_runtime() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let bus = FlowBus::new();
    bus.register(
        "retryCmp",
        RetryCmp {
            attempts: attempts.clone(),
        },
    );
    bus.add_chain("retry_chain", "THEN(retryCmp)").unwrap();

    let response = bus.execute("retry_chain").await;

    assert!(response.is_success(), "{}", response.message);
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn fallback_macro_routes_missing_boolean_node_at_runtime() {
    let bus = FlowBus::new();
    BooleanFallbackCmp.register_fallback(&bus).unwrap();
    bus.register(
        "vip",
        liteflow_core::cmp(|ctx| async move {
            ctx.set_data("fallback_plan", json!("vip"));
            Ok(Value::Null)
        }),
    );
    bus.register(
        "normal",
        liteflow_core::cmp(|ctx| async move {
            ctx.set_data("fallback_plan", json!("normal"));
            Ok(Value::Null)
        }),
    );
    bus.add_chain("fallback_chain", "IF(missingBooleanCmp, vip, normal)")
        .unwrap();

    let response = bus.execute("fallback_chain").await;

    assert!(response.is_success(), "{}", response.message);
    assert_eq!(
        response.data("fallback_expected_id"),
        Some(json!("missingBooleanCmp"))
    );
    assert_eq!(response.data("fallback_plan"), Some(json!("vip")));
    assert!(bus.contains_fallback(liteflow_core::NodeTypeEnum::Boolean));
}

#[test]
fn marker_macros_preserve_items_and_metadata() {
    assert_eq!(BooleanFallbackCmp::LITEFLOW_FALLBACK_TYPE, "boolean");
    assert_eq!(BooleanFallbackCmp::LITEFLOW_FALLBACK_ID, "booleanFallback");
    assert_eq!(alias_target(), 7);
}
