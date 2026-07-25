//! 对应 CatchCondition：捕获执行 DO；无 DO 则异常继续抛出。

use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub struct CatchCondition {
    catch_item: Arc<dyn Executable>,
    do_item: Option<Arc<dyn Executable>>,
}

impl CatchCondition {
    pub fn new(catch_item: Arc<dyn Executable>, do_item: Option<Arc<dyn Executable>>) -> Self {
        Self { catch_item, do_item }
    }
}

#[async_trait]
impl Executable for CatchCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        match self.catch_item.execute(ctx, frame).await {
            Ok(v) => Ok(v),
            Err(LiteflowError::ChainEnd) => Err(LiteflowError::ChainEnd),
            Err(e) => {
                ctx.set_exception(&e.to_string());
                match &self.do_item {
                    Some(d) => d.execute(ctx, frame).await,
                    None => Err(e),
                }
            }
        }
    }

    fn id(&self) -> &str {
        "CATCH"
    }
}
