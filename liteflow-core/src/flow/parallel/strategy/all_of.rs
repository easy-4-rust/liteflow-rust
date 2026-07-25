//! 对应 AllOfParallelExecutor：等待全部分支完成。

use super::{collect, spawn_all, ParallelOpts, ParallelStrategyExecutor};
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub struct AllOfParallelExecutor;

#[async_trait]
impl ParallelStrategyExecutor for AllOfParallelExecutor {
    async fn execute(
        &self,
        items: Vec<Arc<dyn Executable>>,
        opts: &ParallelOpts,
        ctx: Ctx,
        frame: Frame,
    ) -> LFResult<Value> {
        let n = items.len();
        let set = spawn_all(items, &ctx, &frame);
        let (out, _) = collect(set, &opts.must_idx, |out| out.oks.len() >= n).await;
        if out.chain_end {
            return Err(LiteflowError::ChainEnd);
        }
        if opts.ignore_error {
            return Ok(Value::Null);
        }
        if out.oks.len() >= n {
            return Ok(Value::Null);
        }
        match out.first_err {
            Some(e) => Err(LiteflowError::WhenExecute(e.to_string())),
            None => Ok(Value::Null),
        }
    }
}
