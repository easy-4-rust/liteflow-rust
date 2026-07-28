//! 等待指定 must 分支全部成功。
//!
//! 对应 Java:
//! `com.yomahub.liteflow.flow.parallel.strategy.SpecifyParallelExecutor`。

use super::{ParallelOpts, ParallelStrategyExecutor, collect, record_timeout_items, spawn_all};
use crate::exception::LFResult;
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// 完成指定任务的并行策略执行器。
///
/// 对应 Java: `com.yomahub.liteflow.flow.parallel.strategy.SpecifyParallelExecutor`。
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
        let n = items.len();
        let must_idx = opts.must_idx.clone();
        let set = spawn_all(items, &ctx, &frame, &opts.executor_service, opts.max_wait);
        let (out, _) = collect(set, &must_idx, |out| {
            if must_idx.is_empty() {
                // Java 在 MUST 指定项一个也不存在时回退为等待全部任务。
                out.completed.len() >= n
            } else {
                must_idx.iter().all(|index| out.completed.contains(index))
            }
        })
        .await;
        record_timeout_items(&out, &ctx);
        if opts.ignore_error {
            return Ok(Value::Null);
        }
        if let Some(error) = out.first_err {
            return Err(error);
        }
        Ok(Value::Null)
    }
}
