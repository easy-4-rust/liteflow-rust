//! 等待全部并行分支完成。
//!
//! 对应 Java:
//! `com.yomahub.liteflow.flow.parallel.strategy.AllOfParallelExecutor`。

use super::{ParallelOpts, ParallelStrategyExecutor, collect, spawn_all};
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// 完成全部任务的并行策略执行器。
///
/// 对应 Java: `com.yomahub.liteflow.flow.parallel.strategy.AllOfParallelExecutor`。
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
        let set = spawn_all(items, &ctx, &frame, &opts.executor_service);
        let (out, _) = collect(set, &opts.must_idx, |out| out.oks.len() >= n).await;
        if out.chain_end {
            return Err(LiteflowError::ChainEnd("chain end".to_string()));
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
