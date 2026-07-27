//! 等待指定 must 分支全部成功。
//!
//! 对应 Java:
//! `com.yomahub.liteflow.flow.parallel.strategy.SpecifyParallelExecutor`。

use super::{ParallelOpts, ParallelStrategyExecutor, collect, spawn_all};
use crate::exception::{LFResult, LiteflowError};
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
        let must_idx = opts.must_idx.clone();
        let set = spawn_all(items, &ctx, &frame, &opts.executor_service);
        let (out, early) = collect(set, &must_idx, |out| {
            must_idx.iter().all(|m| out.oks.contains(m))
        })
        .await;
        if early {
            return Ok(Value::Null);
        }
        if out.chain_end {
            return Err(LiteflowError::ChainEnd("chain end".to_string()));
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
