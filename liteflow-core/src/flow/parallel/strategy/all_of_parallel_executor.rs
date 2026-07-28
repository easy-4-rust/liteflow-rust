//! 等待全部并行分支完成。
//!
//! 对应 Java:
//! `com.yomahub.liteflow.flow.parallel.strategy.AllOfParallelExecutor`。

use super::{ParallelOpts, ParallelStrategyExecutor, collect, record_timeout_items, spawn_all};
use crate::exception::LFResult;
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
        let set = spawn_all(items, &ctx, &frame, &opts.executor_service, opts.max_wait);
        let (out, _) = collect(set, &opts.must_idx, |out| out.completed.len() >= n).await;
        record_timeout_items(&out, &ctx);
        if opts.ignore_error {
            return Ok(Value::Null);
        }
        match out.first_err {
            Some(error) => Err(error),
            None => Ok(Value::Null),
        }
    }
}
