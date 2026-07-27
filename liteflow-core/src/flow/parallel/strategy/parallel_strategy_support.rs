//! 并行策略共享执行函数。

use std::collections::HashSet;
use std::sync::Arc;

use serde_json::Value;
use tokio::task::JoinSet;

use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::Executable;
use crate::slot::{Ctx, Frame};
use crate::thread::ExecutorService;

use super::ParallelOutcome;

/// 把全部分支提交到 Tokio JoinSet。
pub fn spawn_all(
    items: Vec<Arc<dyn Executable>>,
    ctx: &Ctx,
    frame: &Frame,
    executor_service: &Arc<ExecutorService>,
) -> JoinSet<(usize, LFResult<Value>)> {
    let mut set = JoinSet::new();
    for (index, item) in items.into_iter().enumerate() {
        let ctx = ctx.clone();
        let frame = frame.clone();
        let executor_service = executor_service.clone();
        set.spawn(async move {
            let result = executor_service
                .execute(async { item.execute(&ctx, &frame).await })
                .await
                .and_then(|result| result);
            (index, result)
        });
    }
    set
}

/// 收集全部分支结果，并允许策略提前完成。
pub async fn collect<F>(
    mut set: JoinSet<(usize, LFResult<Value>)>,
    must_idx: &HashSet<usize>,
    mut early_ok: F,
) -> (ParallelOutcome, bool)
where
    F: FnMut(&ParallelOutcome) -> bool,
{
    let mut outcome = ParallelOutcome {
        oks: HashSet::new(),
        first_err: None,
        chain_end: false,
        must_err: None,
    };
    while let Some(joined) = set.join_next().await {
        let (index, result) = match joined {
            Ok(result) => result,
            Err(error) => {
                if outcome.first_err.is_none() {
                    outcome.first_err = Some(LiteflowError::WhenExecute(error.to_string()));
                }
                continue;
            }
        };
        match result {
            Ok(_) => {
                outcome.oks.insert(index);
                if early_ok(&outcome) {
                    return (outcome, true);
                }
            }
            Err(LiteflowError::ChainEnd(_)) => outcome.chain_end = true,
            Err(error) => {
                if must_idx.contains(&index) && outcome.must_err.is_none() {
                    outcome.must_err = Some(error.clone());
                }
                if outcome.first_err.is_none() {
                    outcome.first_err = Some(error);
                }
            }
        }
        if set.is_empty() {
            break;
        }
    }
    (outcome, false)
}
