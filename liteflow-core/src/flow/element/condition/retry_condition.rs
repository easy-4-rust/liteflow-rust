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
    retry_for: Vec<String>,
}

impl RetryCondition {
    pub fn new(inner: Arc<dyn Executable>, retry_count: u32) -> Self {
        Self {
            inner,
            retry_count,
            retry_for: Vec::new(),
        }
    }

    /// 创建带错误类型过滤的重试条件。
    /// 对应 Java: `RetryCondition#setRetryForExceptions`。
    pub fn with_exceptions(
        inner: Arc<dyn Executable>,
        retry_count: u32,
        retry_for: Vec<String>,
    ) -> Self {
        Self {
            inner,
            retry_count,
            retry_for,
        }
    }

    fn matches_retry_filter(&self, error: &LiteflowError) -> bool {
        if self.retry_for.is_empty() {
            return true;
        }
        let debug;
        let variant = match error {
            LiteflowError::NodeExec { kind, .. } => kind.as_str(),
            other => {
                debug = format!("{other:?}");
                debug.split([' ', '(', '{']).next().unwrap_or_default()
            }
        }
        .trim_end_matches("Exception");
        self.retry_for.iter().any(|name| {
            let simple = name.rsplit('.').next().unwrap_or(name);
            simple.trim_end_matches("Exception") == variant
                || simple == "LiteflowError"
                || simple == "Error"
        })
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
                Err(e) if self.matches_retry_filter(&e) => last_err = Some(e),
                Err(e) => return Err(e),
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
