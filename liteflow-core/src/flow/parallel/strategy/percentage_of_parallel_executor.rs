//! 达到指定完成比例后结束等待，阈值数量向上取整。
//!
//! 对应 Java:
//! `com.yomahub.liteflow.flow.parallel.strategy.PercentageOfParallelExecutor`。

use super::{ParallelOpts, ParallelStrategyExecutor, collect, record_timeout_items, spawn_all};
use crate::exception::LFResult;
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// 完成指定阈值任务的并行策略执行器。
///
/// 对应 Java:
/// `com.yomahub.liteflow.flow.parallel.strategy.PercentageOfParallelExecutor`。
pub struct PercentageOfParallelExecutor;

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
        // Java 执行器从 WhenCondition 读取 percentage；Rust 通过不可变选项传入，
        // 从而让执行器自身可以安全缓存和跨并发执行复用。
        let percentage = opts.percentage.unwrap_or(1.0);
        let need = ((percentage * n as f64).ceil() as usize).max(1);
        let set = spawn_all(items, &ctx, &frame, &opts.executor_service, opts.max_wait);
        let (out, _) = collect(set, &opts.must_idx, |out| out.completed.len() >= need).await;
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
