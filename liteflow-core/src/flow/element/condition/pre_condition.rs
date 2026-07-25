//! 对应 PreCondition。

use crate::exception::LFResult;
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub struct PreCondition {
    item: Arc<dyn Executable>,
}

impl PreCondition {
    pub fn new(item: Arc<dyn Executable>) -> Self {
        Self { item }
    }
}

#[async_trait]
impl Executable for PreCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        self.item.execute(ctx, frame).await
    }
    fn id(&self) -> &str {
        "PRE"
    }
    fn is_pre_or_finally(&self) -> bool {
        true
    }
}
