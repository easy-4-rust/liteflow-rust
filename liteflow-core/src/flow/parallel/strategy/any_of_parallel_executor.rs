//! 任一并行分支成功即返回。
//!
//! 对应 Java:
//! `com.yomahub.liteflow.flow.parallel.strategy.AnyOfParallelExecutor`。

use super::{ParallelOpts, ParallelStrategyExecutor, collect, spawn_all};
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// 完成任一任务的并行策略执行器。
///
/// 对应 Java: `com.yomahub.liteflow.flow.parallel.strategy.AnyOfParallelExecutor`。
pub struct AnyOfParallelExecutor;

#[async_trait]
impl ParallelStrategyExecutor for AnyOfParallelExecutor {
    async fn execute(
        &self,
        items: Vec<Arc<dyn Executable>>,
        opts: &ParallelOpts,
        ctx: Ctx,
        frame: Frame,
    ) -> LFResult<Value> {
        let set = spawn_all(items, &ctx, &frame, &opts.executor_service);
        let (out, early) = collect(set, &opts.must_idx, |out| !out.oks.is_empty()).await;
        if early {
            return Ok(Value::Null); // 任一成功即返回，其余分支随 JoinSet drop 取消
        }
        if out.chain_end {
            return Err(LiteflowError::ChainEnd("chain end".to_string()));
        }
        if !out.oks.is_empty() || opts.ignore_error {
            return Ok(Value::Null);
        }
        match out.first_err {
            Some(e) => Err(LiteflowError::WhenExecute(e.to_string())),
            None => Ok(Value::Null),
        }
    }
}
