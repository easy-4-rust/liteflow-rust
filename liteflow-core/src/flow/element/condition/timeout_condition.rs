//! 对应 TimeoutCondition：MAX_WAIT_SECONDS / MAX_WAIT_MILLISECONDS。

use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

pub struct TimeoutCondition {
    inner: Arc<dyn Executable>,
    max_wait_ms: u64,
}

impl TimeoutCondition {
    pub fn new(inner: Arc<dyn Executable>, max_wait_ms: u64) -> Self {
        Self { inner, max_wait_ms }
    }
}

#[async_trait]
impl Executable for TimeoutCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        match tokio::time::timeout(
            Duration::from_millis(self.max_wait_ms),
            self.inner.execute(ctx, frame),
        )
        .await
        {
            Ok(r) => r,
            Err(_) => Err(LiteflowError::WhenTimeout),
        }
    }

    fn id(&self) -> &str {
        "TIMEOUT"
    }
}
