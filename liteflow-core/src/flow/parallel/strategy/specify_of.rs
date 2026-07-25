//! 对应 SpecifyParallelExecutor：指定的 must 分支全部成功即完成。

use super::{collect, spawn_all, ParallelOpts, ParallelStrategyExecutor};
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub struct SpecifyParallelExecutor;

#[async_trait]
impl ParallelStrategyExecutor for SpecifyParallelExecutor {
    async fn execute(
        &self,
        items: Vec<Arc<dyn Executable>>,
        opts: &ParallelOpts,
        ctx: Ctx,
        frame: Frame,
    ) -> LFResult<Value> {
        let must_idx = opts.must_idx.clone();
        let set = spawn_all(items, &ctx, &frame);
        let (out, early) = collect(set, &must_idx, |out| {
            must_idx.iter().all(|m| out.oks.contains(m))
        })
        .await;
        if early {
            return Ok(Value::Null);
        }
        if out.chain_end {
            return Err(LiteflowError::ChainEnd);
        }
        if let Some(e) = out.must_err {
            return Err(LiteflowError::WhenExecute(e.to_string()));
        }
        if must_idx.iter().all(|m| out.oks.contains(m)) {
            return Ok(Value::Null);
        }
        Err(LiteflowError::WhenExecute(
            "specified parallel items not completed".into(),
        ))
    }
}
