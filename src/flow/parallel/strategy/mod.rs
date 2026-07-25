//! 对应 flow.parallel.strategy 包。

pub mod all_of;
pub mod any_of;
pub mod percentage_of;
pub mod specify_of;

use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::task::JoinSet;

/// 并行分支结算结果
pub struct ParallelOutcome {
    /// 成功分支序号
    pub oks: HashSet<usize>,
    /// 首个错误
    pub first_err: Option<LiteflowError>,
    /// 是否有分支触发 ChainEnd
    pub chain_end: bool,
    /// 指定序号的错误（SPECIFY 策略用）
    pub must_err: Option<LiteflowError>,
}

/// 并行等待的公共选项（对应 WhenCondition 字段）
pub struct ParallelOpts {
    pub ignore_error: bool,
    pub must_idx: HashSet<usize>,
}

/// 并行执行器统一接口（对应 ParallelStrategyExecutor）
#[async_trait::async_trait]
pub trait ParallelStrategyExecutor: Send + Sync {
    async fn execute(
        &self,
        items: Vec<Arc<dyn Executable>>,
        opts: &ParallelOpts,
        ctx: Ctx,
        frame: Frame,
    ) -> LFResult<Value>;
}

/// 把分支全部提交到 JoinSet（对齐 Java CompletableFuture.supplyAsync 提交）
pub fn spawn_all(
    items: Vec<Arc<dyn Executable>>,
    ctx: &Ctx,
    frame: &Frame,
) -> JoinSet<(usize, LFResult<Value>)> {
    let mut set = JoinSet::new();
    for (i, item) in items.into_iter().enumerate() {
        let ctx = ctx.clone();
        let frame = frame.clone();
        set.spawn(async move { (i, item.execute(&ctx, &frame).await) });
    }
    set
}

/// 收集全部结算结果（带提前完成判定）
pub async fn collect<F>(
    mut set: JoinSet<(usize, LFResult<Value>)>,
    must_idx: &HashSet<usize>,
    mut early_ok: F,
) -> (ParallelOutcome, bool)
where
    F: FnMut(&ParallelOutcome) -> bool,
{
    let mut out = ParallelOutcome {
        oks: HashSet::new(),
        first_err: None,
        chain_end: false,
        must_err: None,
    };
    while let Some(joined) = set.join_next().await {
        let (i, res) = match joined {
            Ok(x) => x,
            Err(join_err) => {
                if out.first_err.is_none() {
                    out.first_err = Some(LiteflowError::WhenExecute(join_err.to_string()));
                }
                continue;
            }
        };
        match res {
            Ok(_) => {
                out.oks.insert(i);
                if early_ok(&out) {
                    return (out, true);
                }
            }
            Err(LiteflowError::ChainEnd) => {
                out.chain_end = true;
            }
            Err(e) => {
                if must_idx.contains(&i) && out.must_err.is_none() {
                    out.must_err = Some(e.clone());
                }
                if out.first_err.is_none() {
                    out.first_err = Some(e);
                }
            }
        }
        if set.is_empty() {
            break;
        }
    }
    (out, false)
}
