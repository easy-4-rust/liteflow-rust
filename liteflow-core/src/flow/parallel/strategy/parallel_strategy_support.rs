//! 并行策略共享执行函数。

use std::collections::HashSet;
use std::sync::Arc;

use tokio::task::JoinSet;

use crate::exception::LiteflowError;
use crate::flow::element::Executable;
use crate::flow::parallel::WhenFutureObj;
use crate::slot::{Ctx, Frame};
use crate::thread::ExecutorService;

use super::ParallelOutcome;

/// 把已结算的逐分支超时写入当前 Slot。
///
/// 参数 `outcome` 是策略已观察到的分支结果，`ctx` 提供当前 Slot；无返回值。
/// 对应 Java: `ParallelStrategyExecutor#handleTaskResult` 中
/// `slot.addTimeoutItem(whenFutureObj.getExecutorId())`。
pub fn record_timeout_items(outcome: &ParallelOutcome, ctx: &Ctx) {
    let mut timeout_items = outcome.timeout_items.clone();
    timeout_items.sort_by_key(|(index, _)| *index);
    for (_, executor_id) in timeout_items {
        ctx.inner.add_timeout_item(executor_id);
    }
}

/// 把全部分支提交到 Tokio JoinSet。
///
/// 参数 `items`、`ctx`、`frame`、`executor_service` 和 `max_wait` 分别对应 Java
/// 的执行项、slot/当前链上下文、并行线程池与单分支等待时长；返回的 JoinSet
/// 为每个分支生成一个 `WhenFutureObj`。对应 Java:
/// `ParallelStrategyExecutor#getWhenAllTaskList/wrappedFutureObj`。
pub fn spawn_all(
    items: Vec<Arc<dyn Executable>>,
    ctx: &Ctx,
    frame: &Frame,
    executor_service: &Arc<ExecutorService>,
    max_wait: std::time::Duration,
) -> JoinSet<(usize, WhenFutureObj)> {
    let mut set = JoinSet::new();
    for (index, item) in items.into_iter().enumerate() {
        let executor_id = item.id().to_string();
        let ctx = ctx.clone();
        let frame = frame.clone();
        let executor_service = executor_service.clone();
        // Java 的 completeOnTimeout 只完成包装 Future，无法停止底层线程。这里先
        // 独立提交真实执行任务；包装任务超时或被策略丢弃时，JoinHandle 被丢弃但
        // 底层任务继续运行，从而保持相同的取消边界。
        let mut execution = tokio::spawn(async move {
            executor_service
                .execute(async { item.execute(&ctx, &frame).await })
                .await
                .and_then(|result| result)
        });
        set.spawn(async move {
            let when_future_obj = match tokio::time::timeout(max_wait, &mut execution).await {
                Ok(Ok(Ok(_))) => WhenFutureObj::success(&executor_id),
                Ok(Ok(Err(error))) => WhenFutureObj::fail(&executor_id, error),
                Ok(Err(error)) => {
                    WhenFutureObj::fail(&executor_id, LiteflowError::WhenExecute(error.to_string()))
                }
                Err(_) => WhenFutureObj::time_out(&executor_id),
            };
            (index, when_future_obj)
        });
    }
    set
}

/// 收集分支结果，并允许调用方按策略提前完成。
///
/// 参数 `set` 是全部分支包装任务，`must_idx` 是 MUST 分支序号，`early_ok`
/// 判断策略门闩是否已经打开；返回已观察结果及是否提前结束。对应 Java:
/// `ParallelStrategyExecutor#handleTaskResult`。
pub async fn collect<F>(
    mut set: JoinSet<(usize, WhenFutureObj)>,
    must_idx: &HashSet<usize>,
    mut early_ok: F,
) -> (ParallelOutcome, bool)
where
    F: FnMut(&ParallelOutcome) -> bool,
{
    let mut outcome = ParallelOutcome {
        completed: HashSet::new(),
        oks: HashSet::new(),
        timeout_items: Vec::new(),
        first_err: None,
        first_err_index: None,
        chain_end: false,
        must_err: None,
        must_err_index: None,
    };
    while let Some(joined) = set.join_next().await {
        let (index, when_future_obj) = match joined {
            Ok(result) => result,
            Err(error) => {
                if outcome.first_err.is_none() {
                    outcome.first_err = Some(LiteflowError::WhenExecute(error.to_string()));
                }
                continue;
            }
        };
        apply_result(&mut outcome, must_idx, index, when_future_obj);
        if early_ok(&outcome) {
            // Java handleTaskResult 会读取所有在门闩打开瞬间已经完成的 Future；
            // 因此在结束等待前，把 Tokio JoinSet 中已经就绪的结果一并结算。
            while let Some(joined) = set.try_join_next() {
                match joined {
                    Ok((index, when_future_obj)) => {
                        apply_result(&mut outcome, must_idx, index, when_future_obj);
                    }
                    Err(error) if outcome.first_err.is_none() => {
                        outcome.first_err = Some(LiteflowError::WhenExecute(error.to_string()));
                    }
                    Err(_) => {}
                }
            }
            return (outcome, true);
        }
        if set.is_empty() {
            break;
        }
    }
    (outcome, false)
}

fn apply_result(
    outcome: &mut ParallelOutcome,
    must_idx: &HashSet<usize>,
    index: usize,
    when_future_obj: WhenFutureObj,
) {
    outcome.completed.insert(index);
    if when_future_obj.is_timeout() {
        outcome
            .timeout_items
            .push((index, when_future_obj.get_executor_id().to_string()));
    }
    if when_future_obj.is_success() {
        outcome.oks.insert(index);
        return;
    }
    if let Some(error) = when_future_obj.get_ex().cloned() {
        if matches!(error, LiteflowError::ChainEnd(_)) {
            outcome.chain_end = true;
        }
        if must_idx.contains(&index)
            && outcome
                .must_err_index
                .is_none_or(|first_index| index < first_index)
        {
            outcome.must_err_index = Some(index);
            outcome.must_err = Some(error.clone());
        }
        if outcome
            .first_err_index
            .is_none_or(|first_index| index < first_index)
        {
            outcome.first_err_index = Some(index);
            outcome.first_err = Some(error);
        }
    }
}
