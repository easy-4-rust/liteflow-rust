//! 对应 PercentageOfParallelExecutor：按比例成功即完成（向上取整）。

use super::{collect, spawn_all, ParallelOpts, ParallelStrategyExecutor};
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub struct PercentageOfParallelExecutor {
    pub percentage: f64,
}

#[async_trait]
impl ParallelStrategyExecutor for PercentageOfParallelExecutor {
    async fn execute(
        &self,
        items: Vec<Arc<dyn Executable>>,
        opts: &ParallelOpts,
        ctx: Ctx,
        frame: Frame,
    ) -> LFResult<Value> {
        let n = items.len();
        let need = ((self.percentage * n as f64).ceil() as usize).max(1);
        let set = spawn_all(items, &ctx, &frame);
        let (out, early) = collect(set, &opts.must_idx, |out| out.oks.len() >= need).await;
        if early {
            return Ok(Value::Null);
        }
        if out.chain_end {
            return Err(LiteflowError::ChainEnd);
        }
        if out.oks.len() >= need || opts.ignore_error {
            return Ok(Value::Null);
        }
        match out.first_err {
            Some(e) => Err(LiteflowError::WhenExecute(e.to_string())),
            None => Ok(Value::Null),
        }
    }
}
