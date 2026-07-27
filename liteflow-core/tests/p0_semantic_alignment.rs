//! LiteFlow Java 核心执行语义回归测试。
//!
//! 对应 Java:
//! - `com.yomahub.liteflow.core.FlowExecutor#doExecute`
//! - `com.yomahub.liteflow.core.FlowExecutor#doExecuteWithRoute`
//! - `com.yomahub.liteflow.core.NodeComponent#execute`
//! - `com.yomahub.liteflow.core.NodeComponent#doRollback`

use async_trait::async_trait;
use liteflow_core::aop::ICmpAroundAspect;
use liteflow_core::lifecycle::PostProcessFlowExecuteLifeCycle;
use liteflow_core::{CmpContext, FlowBus, LiteflowError, NodeComponent, rule};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct OrderedComponent {
    events: Arc<Mutex<Vec<String>>>,
    fail: bool,
}

#[async_trait]
impl NodeComponent for OrderedComponent {
    async fn before_process(&self, _ctx: &CmpContext) -> Result<(), LiteflowError> {
        self.events.lock().unwrap().push("component_before".into());
        Ok(())
    }

    async fn process(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        self.events.lock().unwrap().push("component_process".into());
        if self.fail {
            Err(LiteflowError::Custom("boom".into()))
        } else {
            Ok(Value::Null)
        }
    }

    async fn on_success(&self, _ctx: &CmpContext) -> Result<(), LiteflowError> {
        self.events.lock().unwrap().push("component_success".into());
        Ok(())
    }

    async fn on_error(&self, _ctx: &CmpContext, _e: &LiteflowError) {
        self.events.lock().unwrap().push("component_error".into());
    }

    async fn after_process(&self, _ctx: &CmpContext) {
        self.events.lock().unwrap().push("component_after".into());
    }
}

struct OrderedAspect {
    events: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl ICmpAroundAspect for OrderedAspect {
    async fn before_process(&self, _ctx: &CmpContext) {
        self.events.lock().unwrap().push("aspect_before".into());
    }

    async fn on_success(&self, _ctx: &CmpContext) {
        self.events.lock().unwrap().push("aspect_success".into());
    }

    async fn on_error(&self, _ctx: &CmpContext, _e: &LiteflowError) {
        self.events.lock().unwrap().push("aspect_error".into());
    }

    async fn after_process(&self, _ctx: &CmpContext) {
        self.events.lock().unwrap().push("aspect_after".into());
    }
}

#[tokio::test]
async fn component_hooks_follow_java_success_order() {
    let bus = FlowBus::new();
    let events = Arc::new(Mutex::new(Vec::new()));
    bus.register_aspect(Arc::new(OrderedAspect {
        events: events.clone(),
    }));
    bus.register(
        "ordered",
        OrderedComponent {
            events: events.clone(),
            fail: false,
        },
    );
    bus.add_chain("c1", "THEN(ordered)").unwrap();

    assert!(bus.execute("c1").await.is_success());
    assert_eq!(
        *events.lock().unwrap(),
        [
            "aspect_before",
            "component_before",
            "component_process",
            "component_success",
            "aspect_success",
            "component_after",
            "aspect_after",
        ]
    );
}

#[tokio::test]
async fn component_hooks_follow_java_error_order() {
    let bus = FlowBus::new();
    let events = Arc::new(Mutex::new(Vec::new()));
    bus.register_aspect(Arc::new(OrderedAspect {
        events: events.clone(),
    }));
    bus.register(
        "ordered",
        OrderedComponent {
            events: events.clone(),
            fail: true,
        },
    );
    bus.add_chain("c1", "THEN(ordered)").unwrap();

    assert!(!bus.execute("c1").await.is_success());
    assert_eq!(
        *events.lock().unwrap(),
        [
            "aspect_before",
            "component_before",
            "component_process",
            "component_error",
            "aspect_error",
            "component_after",
            "aspect_after",
        ]
    );
}

struct RollbackComponent {
    id: &'static str,
    events: Arc<Mutex<Vec<String>>>,
    fail: bool,
}

#[async_trait]
impl NodeComponent for RollbackComponent {
    async fn process(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        self.events
            .lock()
            .unwrap()
            .push(format!("execute:{}", self.id));
        if self.fail {
            Err(LiteflowError::Custom(format!("{} failed", self.id)))
        } else {
            Ok(Value::Null)
        }
    }

    fn is_rollback(&self) -> bool {
        true
    }

    async fn rollback(&self, _ctx: &CmpContext) -> Result<(), LiteflowError> {
        self.events
            .lock()
            .unwrap()
            .push(format!("rollback:{}", self.id));
        Ok(())
    }
}

#[tokio::test]
async fn failed_chain_rolls_back_executed_components_in_reverse_order() {
    let bus = FlowBus::new();
    let events = Arc::new(Mutex::new(Vec::new()));
    for (id, fail) in [("a", false), ("b", false), ("c", true)] {
        bus.register(
            id,
            RollbackComponent {
                id,
                events: events.clone(),
                fail,
            },
        );
    }
    bus.add_chain("c1", "THEN(a, b, c)").unwrap();

    let response = bus.execute("c1").await;
    assert!(!response.is_success());
    assert_eq!(
        *events.lock().unwrap(),
        [
            "execute:a",
            "execute:b",
            "execute:c",
            "rollback:c",
            "rollback:b",
            "rollback:a",
        ]
    );
    assert_eq!(
        response
            .rollback_steps
            .iter()
            .map(|step| step.node_id.as_str())
            .collect::<Vec<_>>(),
        ["c", "b", "a"]
    );
}

struct FlowHook {
    events: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl PostProcessFlowExecuteLifeCycle for FlowHook {
    async fn post_process_before_flow_execute(&self, chain_id: &str) {
        self.events
            .lock()
            .unwrap()
            .push(format!("before:{chain_id}"));
    }

    async fn post_process_after_flow_execute(&self, chain_id: &str) {
        self.events
            .lock()
            .unwrap()
            .push(format!("after:{chain_id}"));
    }
}

impl liteflow_core::LifeCycle for FlowHook {
    fn register_life_cycle(
        self: Arc<Self>,
        life_cycle_holder: &mut liteflow_core::LifeCycleHolder,
    ) {
        life_cycle_holder.flow_execute.push(self);
    }
}

#[tokio::test]
async fn after_lifecycle_runs_when_chain_is_missing() {
    let bus = FlowBus::new();
    let events = Arc::new(Mutex::new(Vec::new()));
    bus.register_flow_execute_hook(Arc::new(FlowHook {
        events: events.clone(),
    }));

    let response = bus.execute("missing").await;
    assert!(!response.is_success());
    assert_eq!(*events.lock().unwrap(), ["before:missing", "after:missing"]);
}

struct RouteRecorder {
    request_ids: Arc<Mutex<Vec<String>>>,
    route: bool,
}

#[async_trait]
impl NodeComponent for RouteRecorder {
    async fn process(&self, ctx: &CmpContext) -> Result<Value, LiteflowError> {
        self.request_ids
            .lock()
            .unwrap()
            .push(ctx.request_id().to_string());
        if self.route {
            Ok(Value::Bool(true))
        } else {
            Ok(Value::Null)
        }
    }
}

#[tokio::test]
async fn route_and_body_share_one_request_id() {
    let bus = FlowBus::new();
    let request_ids = Arc::new(Mutex::new(Vec::new()));
    bus.register(
        "route",
        RouteRecorder {
            request_ids: request_ids.clone(),
            route: true,
        },
    );
    bus.register(
        "body",
        RouteRecorder {
            request_ids: request_ids.clone(),
            route: false,
        },
    );
    rule::load_json_str(
        &bus,
        r#"{"flow":{"chain":[
            {"id":"routeChain","namespace":"ns","route":"route","body":"THEN(body)"}
        ]}}"#,
    )
    .unwrap();

    let responses = bus
        .execute_route_chain(Some("ns"), json!({"order_id": "A001"}))
        .await
        .unwrap();
    let seen = request_ids.lock().unwrap().clone();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0], seen[1]);
    assert_eq!(responses[0].request_id, seen[0]);
}

struct SlowComponent {
    request_id: Arc<Mutex<Option<String>>>,
}

#[async_trait]
impl NodeComponent for SlowComponent {
    async fn process(&self, ctx: &CmpContext) -> Result<Value, LiteflowError> {
        *self.request_id.lock().unwrap() = Some(ctx.request_id().to_string());
        let input: Value = ctx.request_data().unwrap();
        ctx.set_data("observed_input", input);
        tokio::time::sleep(Duration::from_millis(200)).await;
        Ok(Value::Null)
    }
}

#[tokio::test]
async fn timeout_response_keeps_original_slot_correlation() {
    let bus = FlowBus::new();
    let request_id = Arc::new(Mutex::new(None));
    bus.register(
        "slow",
        SlowComponent {
            request_id: request_id.clone(),
        },
    );
    bus.add_chain("c1", "THEN(slow)").unwrap();

    let response = bus
        .execute_timeout("c1", json!({"order_id": "A001"}), Duration::from_millis(30))
        .await;
    assert!(!response.is_success());
    assert_eq!(
        response.request_id,
        request_id.lock().unwrap().clone().unwrap()
    );
    assert_eq!(
        response.data("observed_input"),
        Some(json!({"order_id": "A001"}))
    );
}

#[test]
fn auto_conversation_id_clears_previous_explicit_value() {
    let option = liteflow_core::ExecuteOption::of()
        .conversation_id("explicit")
        .auto_conversation_id();
    assert!(option.conversation_id.is_none());
    assert!(option.auto_conversation_id);
}
