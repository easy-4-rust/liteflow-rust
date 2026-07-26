//! 对应 Java 类：com.yomahub.liteflow.flow.element.condition.LoopCondition
//!
//! FOR/WHILE/ITERATOR 的公共逻辑（并行提交的 BREAK 检查与 future 结算，
//! 对齐 Java handleFutureList）。
//!
//! 架构映射说明：
//! - Java LoopCondition#getBreakNode / #getDoExecutor（按 ConditionKey 取元素）
//!   → Rust 各子类结构体的 break_item / do_executor 字段。
//! - Java #setLoopIndex / #setCurrLoopObject（递归下传 Chain/Condition/Node）
//!   → Rust Frame::push(index, object)，随执行路径 clone 下传，语义等价。

use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::flow::parallel::LoopFutureObj;
use crate::slot::{Ctx, Frame};
use crate::thread::ExecutorService;
use serde_json::Value;
use std::sync::Arc;
use tokio::task::JoinSet;

/// 并行循环：提交一轮迭代任务，然后（对齐 Java）在启动线程检查 BREAK。
/// 返回 false 表示 BREAK，停止后续提交。
pub async fn submit_iteration(
    set: &mut JoinSet<LoopFutureObj>,
    body: &Arc<dyn Executable>,
    brk: Option<&Arc<dyn Executable>>,
    ctx: &Ctx,
    frame: &Frame,
    index: usize,
    object: Option<Value>,
    executor_service: &Arc<ExecutorService>,
) -> LFResult<bool> {
    let body = body.clone();
    let executor_name = body.id().to_string();
    let ctx2 = ctx.clone();
    let f = frame.push(index, object.clone());
    let executor_service = executor_service.clone();
    set.spawn(async move {
        let result = executor_service
            .execute(async { body.execute(&ctx2, &f).await })
            .await
            .and_then(|result| result);
        match result {
            Ok(_) => LoopFutureObj::success(executor_name),
            Err(error) => LoopFutureObj::fail(executor_name, error),
        }
    });

    if let Some(b) = brk {
        let f2 = frame.push(index, object);
        let v = b.execute(ctx, &f2).await?;
        return Ok(!super::expect_bool(b.id(), &v)?);
    }
    Ok(true)
}

/// 结算全部并行循环子项，任一任务失败即返回对应异常。
///
/// 每个任务先封装为 `LoopFutureObj`，保留执行器名称和原始异常，再由此方法统一
/// 转换为循环执行结果。对应 Java: `LoopCondition#handleFutureList`。
pub async fn handle_future_list(mut set: JoinSet<LoopFutureObj>) -> LFResult<Value> {
    while let Some(joined) = set.join_next().await {
        match joined {
            Ok(result) if result.is_success() => {}
            Ok(result) => match result.ex().cloned() {
                Some(LiteflowError::ChainEnd) => return Err(LiteflowError::ChainEnd),
                Some(error) => return Err(LiteflowError::WhenExecute(error.to_string())),
                None => {
                    return Err(LiteflowError::WhenExecute(format!(
                        "parallel loop item[{}] failed without error",
                        result.executor_name()
                    )));
                }
            },
            Err(join_err) => return Err(LiteflowError::WhenExecute(join_err.to_string())),
        }
    }
    Ok(Value::Null)
}

/// 顺序循环体执行 + BREAK 检查
pub async fn run_sequential(
    body: &Arc<dyn Executable>,
    brk: Option<&Arc<dyn Executable>>,
    ctx: &Ctx,
    frame: &Frame,
    index: usize,
    object: Option<Value>,
) -> LFResult<bool> {
    let f = frame.push(index, object);
    body.execute(ctx, &f).await?;
    if let Some(b) = brk {
        let v = b.execute(ctx, &f).await?;
        return Ok(!super::expect_bool(b.id(), &v)?);
    }
    Ok(true)
}
