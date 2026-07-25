//! 对应 NotCondition：布尔取反。

use super::expect_bool;
use crate::exception::LFResult;
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub struct NotCondition {
    item: Arc<dyn Executable>,
}

impl NotCondition {
    pub fn new(item: Arc<dyn Executable>) -> Self {
        Self { item }
    }
}

#[async_trait]
impl Executable for NotCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        let v = self.item.execute(ctx, frame).await?;
        Ok(Value::Bool(!expect_bool(self.item.id(), &v)?))
    }

    fn id(&self) -> &str {
        "NOT"
    }
}
