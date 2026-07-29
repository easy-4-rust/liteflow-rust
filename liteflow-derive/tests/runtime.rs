use std::any::Any;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use liteflow_core::flow::element::node::Node;
use liteflow_core::flow::executor::{NodeExecutor, NodeExecutorHelper};
use liteflow_core::slot::{Ctx, Frame};
use liteflow_core::{CmpContext, FlowBus, LiteflowError, NodeComponent};
use liteflow_derive::{
    alias_for, context_bean, fallback_cmp, liteflow_cmp_define, liteflow_component, liteflow_retry,
    script_bean, script_method,
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
}

/// Java 类级 `@LiteflowCmpDefine(NodeTypeEnum.BOOLEAN)` 的 Rust 对等组件。
struct VipCmp;

#[liteflow_cmp_define("vipCmp", node_name = "会员判断", node_type = "boolean")]
impl VipCmp {
    #[liteflow_method("isVip")]
    async fn is_vip(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(json!(true))
    }
}

/// Java 方法级声明式组件：同一对象按 `nodeId` 生成多个节点。
struct GroupedCmp;

struct CountingNodeExecutor {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl NodeExecutor for CountingNodeExecutor {
    async fn execute(&self, node: &Node, ctx: &Ctx, frame: &Frame) -> Result<Value, LiteflowError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        node.execute_once(ctx, frame).await
    }
}

#[liteflow_cmp_define("groupedFallback")]
impl GroupedCmp {
    #[liteflow_method(value = "before_process", node_id = "groupA", node_name = "分组 A")]
    async fn before_group_a(&self, ctx: &CmpContext) -> Result<Value, LiteflowError> {
        ctx.set_data("group_a_before", json!(true));
        Ok(Value::Null)
    }

    #[liteflow_method(value = "process", node_id = "groupA", node_name = "分组 A")]
    async fn process_group_a(&self, ctx: &CmpContext) -> Result<Value, LiteflowError> {
        ctx.set_data("group_a_process", json!(true));
        Ok(Value::Null)
    }

    #[liteflow_method(value = "on_success", node_id = "groupA")]
    async fn success_group_a(&self, ctx: &CmpContext) -> Result<Value, LiteflowError> {
        ctx.set_data("group_a_success", json!(true));
        Ok(Value::Null)
    }

    #[liteflow_method(value = "after_process", node_id = "groupA")]
    async fn after_group_a(&self, ctx: &CmpContext) -> Result<Value, LiteflowError> {
        ctx.set_data("group_a_after", json!(true));
        Ok(Value::Null)
    }

    #[liteflow_method(value = "get_display_name", node_id = "groupA")]
    async fn display_group_a(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(json!("动态分组 A"))
    }

    #[liteflow_method(value = "process", node_id = "groupB", node_name = "分组 B")]
    async fn process_group_b(&self, ctx: &CmpContext) -> Result<Value, LiteflowError> {
        ctx.set_data("group_b_process", json!(true));
        Ok(Value::Null)
    }

    #[liteflow_method(value = "get_node_executor_class", node_id = "groupB")]
    async fn executor_group_b(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(json!("derive.CountingNodeExecutor"))
    }

    #[liteflow_method(value = "process", node_id = "groupSkipped")]
    async fn process_skipped(&self, ctx: &CmpContext) -> Result<Value, LiteflowError> {
        ctx.set_data("skipped_process", json!(true));
        Ok(Value::Null)
    }

    #[liteflow_method(value = "is_access", node_id = "groupSkipped")]
    async fn access_skipped(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(json!(false))
    }

    #[liteflow_method(value = "process", node_id = "groupError")]
    async fn process_error(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Err(LiteflowError::Parse("expected grouped error".to_string()))
    }

    #[liteflow_method(value = "on_error", node_id = "groupError")]
    async fn on_group_error(
        &self,
        ctx: &CmpContext,
        error: &LiteflowError,
    ) -> Result<Value, LiteflowError> {
        ctx.set_data("group_error_hook", json!(true));
        ctx.set_data("group_error_message", json!(error.to_string()));
        Ok(Value::Null)
    }

    #[liteflow_method(value = "is_continue_on_error", node_id = "groupError")]
    async fn continue_group_error(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(json!(true))
    }

    #[liteflow_method(value = "process", node_id = "groupEnd")]
    async fn process_end(&self, ctx: &CmpContext) -> Result<Value, LiteflowError> {
        ctx.set_data("group_end_process", json!(true));
        Ok(Value::Null)
    }

    #[liteflow_method(value = "is_end", node_id = "groupEnd")]
    async fn end_after_process(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(json!(true))
    }

    #[liteflow_method(value = "process", node_id = "groupRollback")]
    async fn process_rollback(&self, ctx: &CmpContext) -> Result<Value, LiteflowError> {
        ctx.set_data("group_rollback_process", json!(true));
        Ok(Value::Null)
    }

    #[liteflow_method(value = "rollback", node_id = "groupRollback")]
    async fn rollback_group(&self, ctx: &CmpContext) -> Result<Value, LiteflowError> {
        ctx.set_data("group_rollback_called", json!(true));
        Ok(Value::Null)
    }
}

struct RetryCmp {
    attempts: Arc<AtomicUsize>,
}

struct DeclRetryCmp {
    attempts: Arc<AtomicUsize>,
}

struct InvalidControlCmp;

struct AccessBooleanCmp;

#[liteflow_cmp_define("booleanFallback", node_type = "boolean")]
impl AccessBooleanCmp {
    #[liteflow_method(value = "process_boolean", node_id = "activeBoolean")]
    async fn active(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(json!(true))
    }

    #[liteflow_method(value = "process_boolean", node_id = "skippedBoolean")]
    async fn skipped(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(json!(false))
    }

    #[liteflow_method(value = "is_access", node_id = "skippedBoolean")]
    async fn skipped_access(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(json!(false))
    }
}

#[liteflow_cmp_define("invalidControlFallback")]
impl InvalidControlCmp {
    #[liteflow_method(value = "process", node_id = "invalidAccess")]
    async fn process_access(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(Value::Null)
    }

    #[liteflow_method(value = "is_access", node_id = "invalidAccess")]
    async fn invalid_access(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(json!("not-a-boolean"))
    }

    #[liteflow_method(value = "process", node_id = "invalidEnd")]
    async fn process_end(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(Value::Null)
    }

    #[liteflow_method(value = "is_end", node_id = "invalidEnd")]
    async fn invalid_end(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(json!(7))
    }

    #[liteflow_method(value = "process", node_id = "invalidContinue")]
    async fn process_continue(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Err(LiteflowError::Parse("primary failure".to_string()))
    }

    #[liteflow_method(value = "is_continue_on_error", node_id = "invalidContinue")]
    async fn invalid_continue(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(Value::Null)
    }

    #[liteflow_method(value = "process", node_id = "invalidDisplay")]
    async fn process_display(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(Value::Null)
    }

    #[liteflow_method(value = "get_display_name", node_id = "invalidDisplay")]
    async fn invalid_display(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(json!(false))
    }

    #[liteflow_method(value = "process", node_id = "invalidExecutor")]
    async fn process_executor(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(Value::Null)
    }

    #[liteflow_method(value = "get_node_executor_class", node_id = "invalidExecutor")]
    async fn invalid_executor(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(json!("example.UnknownNodeExecutor"))
    }
}

#[liteflow_cmp_define("declRetryFallback")]
impl DeclRetryCmp {
    #[liteflow_retry(2, for = ["Parse"])]
    #[liteflow_method(value = "process", node_id = "declRetry")]
    async fn process_retry(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        let attempt = self.attempts.fetch_add(1, Ordering::SeqCst);
        if attempt < 2 {
            return Err(LiteflowError::Parse(
                "temporary declarative error".to_string(),
            ));
        }
        Ok(Value::Null)
    }
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

/// Java `@ScriptBean` 与 `@ScriptMethod` 的 Rust 过程宏映射。
#[script_bean("derive_math", include = "sum", exclude = "hidden")]
struct ScriptMath;

impl ScriptMath {
    #[script_method("sum")]
    fn sum(left: i64, right: i64) -> i64 {
        left + right
    }

    #[script_method]
    fn hidden() -> &'static str {
        "hidden"
    }
}

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
async fn declarative_macro_preserves_class_level_node_types_through_el() {
    let bus = FlowBus::new();
    OrderCmp.register_decl(&bus);
    VipCmp.register_decl(&bus);
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
        "THEN(orderCmp.checkStock, IF(vipCmp.isVip, vip, normal))",
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
    assert_eq!(VipCmp::LITEFLOW_DECL_NODE_TYPE, "boolean");
}

#[tokio::test]
async fn declarative_macro_groups_java_method_metadata_into_lifecycle_nodes() {
    let executor_calls = Arc::new(AtomicUsize::new(0));
    NodeExecutorHelper::load_instance().register_named_node_executor(
        "derive.CountingNodeExecutor",
        Arc::new(CountingNodeExecutor {
            calls: executor_calls.clone(),
        }),
    );
    let bus = FlowBus::new();
    GroupedCmp.register_decl(&bus);
    bus.add_chain(
        "grouped_chain",
        "THEN(groupSkipped, groupA, groupError, groupB)",
    )
    .unwrap();

    let response = bus.execute("grouped_chain").await;

    assert!(response.is_success(), "{}", response.message);
    assert_eq!(response.data("group_a_before"), Some(json!(true)));
    assert_eq!(response.data("group_a_process"), Some(json!(true)));
    assert_eq!(response.data("group_a_success"), Some(json!(true)));
    assert_eq!(response.data("group_a_after"), Some(json!(true)));
    assert_eq!(response.data("group_b_process"), Some(json!(true)));
    assert_eq!(response.data("skipped_process"), None);
    assert_eq!(response.data("group_error_hook"), Some(json!(true)));
    assert_eq!(
        response.data("group_error_message"),
        Some(json!("EL parse error: expected grouped error"))
    );
    assert_eq!(response.steps[0].node_name, "动态分组 A");
    assert!(!response.steps[1].success);
    assert_eq!(response.steps[2].node_name, "分组 B");
    assert_eq!(executor_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        GroupedCmp::LITEFLOW_DECL_NODE_IDS,
        [
            "groupA",
            "groupB",
            "groupEnd",
            "groupError",
            "groupRollback",
            "groupSkipped"
        ]
    );
    assert!(
        NodeExecutorHelper::load_instance()
            .remove_named_node_executor("derive.CountingNodeExecutor")
    );
}

#[tokio::test]
async fn declarative_rollback_runs_when_a_later_node_fails() {
    let bus = FlowBus::new();
    GroupedCmp.register_decl(&bus);
    bus.register(
        "alwaysFail",
        liteflow_core::cmp(|_| async { Err(LiteflowError::Parse("later failure".to_string())) }),
    );
    bus.add_chain("grouped_rollback_chain", "THEN(groupRollback, alwaysFail)")
        .unwrap();

    let response = bus.execute("grouped_rollback_chain").await;

    assert!(!response.is_success());
    assert_eq!(response.data("group_rollback_process"), Some(json!(true)));
    assert_eq!(response.data("group_rollback_called"), Some(json!(true)));
    assert_eq!(response.rollback_steps.len(), 1);
}

#[tokio::test]
async fn declarative_is_end_stops_the_remaining_java_style_nodes() {
    let bus = FlowBus::new();
    GroupedCmp.register_decl(&bus);
    bus.add_chain("grouped_end_chain", "THEN(groupEnd, groupB)")
        .unwrap();

    let response = bus.execute("grouped_end_chain").await;

    assert!(response.is_success(), "{}", response.message);
    assert_eq!(response.data("group_end_process"), Some(json!(true)));
    assert_eq!(response.data("group_b_process"), None);
    assert_eq!(response.steps.len(), 1);
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
async fn declarative_method_retry_reaches_the_real_node_executor() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let bus = FlowBus::new();
    DeclRetryCmp {
        attempts: attempts.clone(),
    }
    .register_decl(&bus);
    bus.add_chain("decl_retry_chain", "THEN(declRetry)")
        .unwrap();

    let response = bus.execute("decl_retry_chain").await;

    assert!(response.is_success(), "{}", response.message);
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn declarative_boolean_controls_reject_non_boolean_results() {
    for (node_id, expected_method) in [
        ("invalidAccess", "isAccess"),
        ("invalidEnd", "isEnd"),
        ("invalidContinue", "isContinueOnError"),
        ("invalidDisplay", "getDisplayName"),
        ("invalidExecutor", "example.UnknownNodeExecutor"),
    ] {
        let bus = FlowBus::new();
        InvalidControlCmp.register_decl(&bus);
        let el = format!("THEN({node_id})");
        bus.add_chain("invalid_control_chain", &el).unwrap();

        let response = bus.execute("invalid_control_chain").await;

        assert!(!response.is_success(), "{node_id} unexpectedly succeeded");
        assert!(
            response.message.contains(expected_method),
            "{}",
            response.message
        );
    }
}

#[tokio::test]
async fn declarative_is_access_filters_and_or_operands_before_execution() {
    let bus = FlowBus::new();
    AccessBooleanCmp.register_decl(&bus);
    bus.register(
        "passed",
        liteflow_core::cmp(|ctx| async move {
            ctx.set_data("and_result", json!("passed"));
            Ok(Value::Null)
        }),
    );
    bus.register(
        "failed",
        liteflow_core::cmp(|ctx| async move {
            ctx.set_data("and_result", json!("failed"));
            Ok(Value::Null)
        }),
    );
    bus.add_chain(
        "decl_access_chain",
        "IF(AND(activeBoolean, skippedBoolean), passed, failed)",
    )
    .unwrap();

    let response = bus.execute("decl_access_chain").await;

    assert!(response.is_success(), "{}", response.message);
    assert_eq!(response.data("and_result"), Some(json!("passed")));
    assert!(
        response
            .steps
            .iter()
            .all(|step| step.node_id != "skippedBoolean")
    );
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

#[tokio::test]
async fn script_annotations_register_filtered_proxy_for_real_rhai_execution() {
    let sum = liteflow_core::script::proxy::ScriptMethodProxy::new(
        ScriptMath::LITEFLOW_SCRIPT_METHOD_SUM,
        Arc::new(|arguments| {
            let left = arguments[0].as_i64().unwrap();
            let right = arguments[1].as_i64().unwrap();
            Ok(json!(ScriptMath::sum(left, right)))
        }),
    );
    let hidden = liteflow_core::script::proxy::ScriptMethodProxy::new(
        ScriptMath::LITEFLOW_SCRIPT_METHOD_HIDDEN,
        Arc::new(|_| Ok(json!(ScriptMath::hidden()))),
    );
    ScriptMath::register_script_bean(vec![sum, hidden]);

    let bus = FlowBus::new();
    bus.register_script(
        "deriveScript",
        "rhai",
        r#"data["derive_sum"] = script_call("derive_math", "sum", [19, 23]);"#,
    )
    .unwrap();
    bus.add_chain("deriveScriptChain", "THEN(deriveScript)")
        .unwrap();

    let response = bus.execute("deriveScriptChain").await;

    assert!(response.is_success(), "{}", response.message);
    assert_eq!(response.data("derive_sum"), Some(json!(42)));
    let proxy = liteflow_core::script::ScriptBeanManager::get_script_bean("derive_math").unwrap();
    assert_eq!(proxy.method_names(), vec!["sum"]);
    liteflow_core::script::ScriptBeanManager::remove_script_bean("derive_math");
}
