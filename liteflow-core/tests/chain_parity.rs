//! Java Chain 对等元数据与执行副作用测试。

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::flow::element::Executable;
use liteflow_core::flow::element::chain::Chain;
use liteflow_core::{Ctx, ExecuteableTypeEnum, Frame, Slot};
use serde_json::{Value, json};

/// 测试用可执行项，返回固定结果并记录执行次数。
struct FixedExecutable {
    id: &'static str,
    result: LFResult<Value>,
    calls: AtomicUsize,
}

impl FixedExecutable {
    /// 创建固定结果的测试执行项。
    fn new(id: &'static str, result: LFResult<Value>) -> Self {
        Self {
            id,
            result,
            calls: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl Executable for FixedExecutable {
    async fn execute(&self, _ctx: &Ctx, _frame: &Frame) -> LFResult<Value> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.result.clone()
    }

    fn id(&self) -> &str {
        self.id
    }
}

/// 验证 Chain 的 Java 命名元数据入口都保存真实状态。
#[test]
#[allow(deprecated)]
fn chain_java_named_metadata_methods_preserve_state() {
    let body: Arc<dyn Executable> = Arc::new(FixedExecutable::new("body", Ok(Value::Null)));
    let route: Arc<dyn Executable> = Arc::new(FixedExecutable::new("route", Ok(Value::Bool(true))));
    let mut chain = Chain::new("draft", Vec::new());

    assert!(!chain.is_compiled());
    chain.set_condition_list(vec![body]);
    chain.set_chain_name("legacy");
    chain.set_chain_id("main");
    chain.set_id("order");
    chain.set_el("THEN(a)");
    chain.set_el_md5("md5");
    chain.set_route_el("route");
    chain.set_route_item(route);
    chain.set_namespace("tenant");
    chain.set_thread_pool_executor_class("pool");
    chain.set_abstract(true);
    chain.set_extends_chain_id("base");
    chain.set_tag("ignored");
    chain.set_compiled(true);

    assert_eq!(chain.get_condition_list().len(), 1);
    assert_eq!(chain.get_chain_name(), "order");
    assert_eq!(chain.get_chain_id(), "order");
    assert_eq!(chain.get_id(), "order");
    assert_eq!(chain.get_el(), Some("THEN(a)"));
    assert_eq!(chain.get_el_md5(), Some("md5"));
    assert_eq!(chain.get_route_el(), Some("route"));
    assert_eq!(chain.get_route_item().map(|item| item.id()), Some("route"));
    assert_eq!(chain.get_namespace(), "tenant");
    assert_eq!(chain.get_thread_pool_executor_class(), Some("pool"));
    assert!(chain.is_abstract());
    assert_eq!(chain.get_extends_chain_id(), Some("base"));
    assert_eq!(chain.get_tag(), None);
    assert!(chain.is_compiled());
    assert_eq!(chain.get_execute_type(), ExecuteableTypeEnum::Chain);
    assert_eq!(
        chain.get_runtime_id(&Frame::root().with_runtime_id(88)),
        Some(88)
    );
}

/// 验证主体异常写入 Slot，而主动 ChainEnd 不作为异常记录。
#[tokio::test]
async fn chain_execute_records_only_real_failures() {
    let failure: Arc<dyn Executable> = Arc::new(FixedExecutable::new(
        "failure",
        Err(LiteflowError::Custom("boom".to_string())),
    ));
    let chain = Chain::new("main", vec![failure]);
    let failure_slot = Arc::new(Slot::new("request-1".to_string(), "main", json!(null)));
    let failure_ctx = Ctx::new(failure_slot.clone());

    assert!(matches!(
        chain.execute(&failure_ctx).await,
        Err(LiteflowError::Custom(message)) if message == "boom"
    ));
    assert_eq!(failure_slot.get_exception(), Some("boom".to_string()));

    let chain_end: Arc<dyn Executable> = Arc::new(FixedExecutable::new(
        "end",
        Err(LiteflowError::ChainEnd("chain end".to_string())),
    ));
    let chain = Chain::new("main", vec![chain_end]);
    let end_slot = Arc::new(Slot::new("request-2".to_string(), "main", json!(null)));
    let end_ctx = Ctx::new(end_slot.clone());

    assert!(matches!(
        chain.execute(&end_ctx).await,
        Err(LiteflowError::ChainEnd(_))
    ));
    assert_eq!(end_slot.get_exception(), None);
}

/// 验证 executeRoute 将布尔结果同步写入 Slot。
#[tokio::test]
async fn chain_execute_route_updates_slot_route_result() {
    let route: Arc<dyn Executable> = Arc::new(FixedExecutable::new("route", Ok(Value::Bool(true))));
    let mut chain = Chain::new("route-chain", Vec::new());
    chain.set_route_item(route);
    let slot = Arc::new(Slot::new(
        "request-3".to_string(),
        "route-chain",
        json!(null),
    ));
    let ctx = Ctx::new(slot.clone());

    assert!(matches!(chain.execute_route(&ctx).await, Ok(true)));
    assert_eq!(slot.get_route_result(), Some(true));
}
