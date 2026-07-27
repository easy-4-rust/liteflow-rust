//! Java `RetryCondition` 字段和真实重试行为回归测试。

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use liteflow_core::flow::element::condition::retry_condition::RetryCondition;
use liteflow_core::flow::element::executable::Executable;
use liteflow_core::slot::{Ctx, Frame, Slot};
use liteflow_core::{LFResult, LiteflowError};
use serde_json::{Value, json};

struct FailOnceExecutable {
    attempts: Arc<AtomicUsize>,
}

#[async_trait]
impl Executable for FailOnceExecutable {
    async fn execute(&self, ctx: &Ctx, _frame: &Frame) -> LFResult<Value> {
        let attempt = self.attempts.fetch_add(1, Ordering::SeqCst);
        if attempt == 0 {
            ctx.set_exception("first attempt failed");
            Err(LiteflowError::Parse("temporary".to_string()))
        } else {
            Ok(json!("recovered"))
        }
    }
}

struct AlwaysFailExecutable {
    attempts: Arc<AtomicUsize>,
}

#[async_trait]
impl Executable for AlwaysFailExecutable {
    async fn execute(&self, _ctx: &Ctx, _frame: &Frame) -> LFResult<Value> {
        self.attempts.fetch_add(1, Ordering::SeqCst);
        Err(LiteflowError::Parse("permanent".to_string()))
    }
}

fn context() -> (Ctx, Frame) {
    (
        Ctx::new(Arc::new(Slot::new(
            "retry-request".to_string(),
            "retry-chain",
            Value::Null,
        ))),
        Frame::root(),
    )
}

#[tokio::test]
async fn java_named_configuration_drives_real_retry_and_clears_slot_error() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let mut condition = RetryCondition::new(
        Arc::new(FailOnceExecutable {
            attempts: Arc::clone(&attempts),
        }),
        0,
    );
    condition.set_retry_times(1);
    condition.set_retry_for_exceptions(vec!["ParseException".to_string()]);

    assert_eq!(condition.get_retry_times(), 1);
    assert_eq!(
        condition.get_retry_for_exceptions(),
        ["ParseException".to_string()]
    );

    let (ctx, frame) = context();
    let result = condition.execute_condition(&ctx, &frame).await.unwrap();
    assert_eq!(result, json!("recovered"));
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    assert!(ctx.inner.get_exception().is_none());
}

#[tokio::test]
async fn empty_exception_filter_prevents_retry_and_negative_times_become_zero() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let mut condition = RetryCondition::with_exceptions(
        Arc::new(AlwaysFailExecutable {
            attempts: Arc::clone(&attempts),
        }),
        3,
        Vec::new(),
    );
    condition.set_retry_times(-1);

    assert_eq!(condition.get_retry_times(), 0);
    assert!(condition.get_retry_for_exceptions().is_empty());

    let (ctx, frame) = context();
    assert!(condition.execute_condition(&ctx, &frame).await.is_err());
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
}
