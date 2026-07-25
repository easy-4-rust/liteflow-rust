//! 对应 RetryCondition：重试 retryCount 次（总尝试 = retryCount + 1）。

use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub struct RetryCondition {
    inner: Arc<dyn Executable>,
    retry_count: u32,
}

impl RetryCondition {
    pub fn new(inner: Arc<dyn Executable>, retry_count: u32) -> Self {
        Self { inner, retry_count }
    }
}

#[async_trait]
impl Executable for RetryCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        let mut last_err: Option<LiteflowError> = None;
        for _ in 0..=self.retry_count {
            match self.inner.execute(ctx, frame).await {
                Ok(v) => return Ok(v),
                Err(LiteflowError::ChainEnd) => return Err(LiteflowError::ChainEnd),
                Err(e) => last_err = Some(e),
            }
        }
        match last_err {
            Some(e) => Err(e),
            None => Ok(Value::Null),
        }
    }

    fn id(&self) -> &str {
        "RETRY"
    }
}
