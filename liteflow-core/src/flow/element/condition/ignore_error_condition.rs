//! 对应 Condition 基类的 ignoreError 语义（非 WHEN 场景）：
//! 包裹一层，吞掉子条件异常并记入 slot。

use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub struct IgnoreErrorCondition {
    inner: Arc<dyn Executable>,
}

impl IgnoreErrorCondition {
    pub fn new(inner: Arc<dyn Executable>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl Executable for IgnoreErrorCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        match self.inner.execute(ctx, frame).await {
            Ok(v) => Ok(v),
            Err(LiteflowError::ChainEnd(message)) => Err(LiteflowError::ChainEnd(message)),
            Err(e) => {
                ctx.set_exception(&e.to_string());
                Ok(Value::Null)
            }
        }
    }

    fn id(&self) -> &str {
        "IGNORE_ERROR"
    }
}
