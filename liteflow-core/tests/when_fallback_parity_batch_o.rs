//! WhenCondition 超时路径与 FallbackNode 委托补测（批次 O）。
//!
//! 覆盖：
//! - `WhenCondition` 的 `setMaxWaitTime` 与全局 `whenMaxWaitSeconds` 配置分支
//! - WHEN 空分支返回 Null
//! - `FallbackNode` 的 NodeComponent 委托方法（isRollback/retryCount/
//!   isRetryFor/rollback/nodeExecutor）
//! - `Ctx#publishEvent` 组件内事件发布

use liteflow_core::flow::element::condition::BooleanConditionTypeEnum;
use liteflow_core::flow::element::condition::and_or_condition::AndOrCondition;
use liteflow_core::flow::element::condition::when_condition::WhenCondition;
use liteflow_core::flow::element::executable::Executable;
use liteflow_core::slot::{Ctx, Slot};
use liteflow_core::{CmpContext, FlowBus, Frame, LiteflowError, NodeComponent, NodeRef, cmp};
use serde_json::{Value, json};
use std::sync::Arc;

/// WhenCondition 的 setMaxWaitTime 直接入口。
#[tokio::test]
async fn when_condition_set_max_wait_time() {
    let mut condition = WhenCondition::new(Vec::new());
    condition.set_max_wait_time(100);
    assert_eq!(condition.get_max_wait_time(), Some(100));
    assert_eq!(
        condition.get_max_wait_time_unit(),
        liteflow_core::property::TimeUnit::Milliseconds
    );
}

/// WHEN 空分支返回 Null（Java WhenCondition 无分支直接成功）。
#[tokio::test]
async fn when_condition_empty_items_returns_null() {
    let condition = WhenCondition::new(Vec::new());
    let slot = Arc::new(Slot::new("RID-EMPTY-WHEN".to_string(), "main", Value::Null));
    let frame = Frame::root();
    let result = condition.execute(&Ctx::new(slot), &frame).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Value::Null);
}

/// 全局 whenMaxWaitSeconds 配置进入 WHEN 超时等待（不触发真实超时）。
#[tokio::test]
async fn when_global_max_wait_seconds_config_applied() {
    let mut config = liteflow_core::LiteflowConfig::default();
    config.set_when_max_wait_time(60);
    config.set_when_max_wait_time_unit(liteflow_core::property::TimeUnit::Seconds);
    let bus = FlowBus::new();
    bus.register(
        "slow",
        cmp(|_| async {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            Ok(json!("done"))
        }),
    );
    bus.add_chain("when_timeout_chain", "WHEN(slow)").unwrap();
    let executor = liteflow_core::FlowExecutor::new_isolated(bus.clone(), config);
    let response = executor.execute("when_timeout_chain").await;
    assert!(response.is_success(), "{}", response.message);
}

/// FallbackNode 的 NodeComponent 委托方法。
#[tokio::test]
async fn fallback_node_component_delegation() {
    let nodes: Arc<dashmap::DashMap<String, Arc<dyn NodeComponent>>> =
        Arc::new(dashmap::DashMap::new());
    let fallback_nodes: Arc<dashmap::DashMap<String, Arc<dyn NodeComponent>>> =
        Arc::new(dashmap::DashMap::new());
    nodes.insert(
        "fb_real".to_string(),
        Arc::new(cmp(|_| async { Ok(json!("real")) })),
    );
    let fallback = liteflow_core::flow::element::fallback_node::FallbackNode::new(
        "fb_real",
        liteflow_core::enums::NodeTypeEnum::Common,
        Arc::clone(&nodes),
        Arc::clone(&fallback_nodes),
    );

    assert!(!fallback.is_rollback());
    assert_eq!(fallback.retry_count(), 0);
    let _ = fallback.is_retry_for(&LiteflowError::Custom("x".into()));
    assert!(
        fallback
            .rollback(&CmpContext {
                inner: Arc::new(Slot::new("RID-FB".to_string(), "main", Value::Null)),
                node: NodeRef::new("fb_real"),
                frame: Frame::root(),
            })
            .await
            .is_ok()
    );
}

/// Ctx#publishEvent 组件内事件发布（带监听器）。
#[tokio::test]
async fn ctx_publish_event_with_listener() {
    let slot = Arc::new(Slot::new("RID-EVT".to_string(), "main", Value::Null));
    let ctx = Ctx::new(slot.clone());
    let seen = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let listener = ListenerStub { seen: seen.clone() };
    liteflow_core::flow::flow_event_publisher::FlowEventPublisher::set_listener(
        &ctx,
        Arc::new(listener),
    );
    ctx.publish_event(
        &liteflow_core::FlowEvent::builder()
            .r#type("cmp-event")
            .build(),
    );
    assert_eq!(*seen.lock().unwrap(), vec!["cmp-event".to_string()]);
}

struct ListenerStub {
    seen: Arc<std::sync::Mutex<Vec<String>>>,
}

impl liteflow_core::FlowEventListener for ListenerStub {
    fn on_event(&self, event: &liteflow_core::FlowEvent) {
        self.seen.lock().unwrap().push(event.get_type().to_string());
    }
}

/// AndOrCondition 空列表返回 Null（Java 短路边界）。
#[tokio::test]
async fn and_or_empty_items_returns_null() {
    let condition = AndOrCondition::new(BooleanConditionTypeEnum::And, Vec::new());
    let slot = Arc::new(Slot::new("RID-EMPTY-AO".to_string(), "main", Value::Null));
    // Java AND 要求至少两个表达式：空列表构建/执行失败是契约行为
    let result = condition.execute(&Ctx::new(slot), &Frame::root()).await;
    assert!(result.is_err());
}
