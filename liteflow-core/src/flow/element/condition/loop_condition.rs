//! 对应 Java 类：com.yomahub.liteflow.flow.element.condition.LoopCondition
//!
//! FOR/WHILE/ITERATOR 的公共状态与执行逻辑（DO/BREAK、线程池、并行提交和
//! future 结算，对齐 Java `LoopCondition` 与内部 `LoopParallelSupplier`）。
//!
//! 架构映射说明：
//! - Java LoopCondition#getBreakNode / #getDoExecutor（按 ConditionKey 取元素）
//!   → Rust 各子类结构体的 break_item / do_executor 字段。
//! - Java #setLoopIndex / #setCurrLoopObject（递归下传 Chain/Condition/Node）
//!   → Rust Frame::push(index, object)，随执行路径 clone 下传，语义等价。

use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::condition::Condition;
use crate::flow::element::executable::Executable;
use crate::flow::parallel::LoopFutureObj;
use crate::slot::{Ctx, Frame};
use crate::thread::ExecutorService;
use serde_json::Value;
use std::sync::Arc;
use tokio::task::JoinSet;

/// 循环 Condition 的公共契约。
///
/// Java 抽象类的字段由 `ForCondition`、`WhileCondition` 和
/// `IteratorCondition` 各自真实持有，本 trait 只提供统一访问入口，不使用全局
/// 旁路状态。对应 Java:
/// `com.yomahub.liteflow.flow.element.condition.LoopCondition`。
pub trait LoopCondition: Condition {
    /// 返回 BREAK 可执行项；未配置时返回 `None`。
    ///
    /// 对应 Java: `LoopCondition#getBreakItem`。
    fn get_break_item(&self) -> Option<&Arc<dyn Executable>>;

    /// 设置 BREAK 可执行项。
    ///
    /// - `break_item`: 每轮循环体之后执行的布尔判断项。
    ///
    /// 对应 Java: `LoopCondition#setBreakItem`。
    fn set_break_item(&mut self, break_item: Arc<dyn Executable>);

    /// 返回循环体可执行项。对应 Java: `LoopCondition#getDoExecutor`。
    fn get_do_executor(&self) -> &Arc<dyn Executable>;

    /// 替换循环体可执行项。
    ///
    /// - `executable`: 新的 DO 主体。
    ///
    /// 对应 Java: `LoopCondition#setDoExecutor`。
    fn set_do_executor(&mut self, executable: Arc<dyn Executable>);

    /// 返回 Condition 级线程池构建器名称。
    ///
    /// 未配置时由 Chain 或全局线程池回退。对应 Java:
    /// `LoopCondition#getThreadPoolExecutorClass`。
    fn get_thread_pool_executor_class(&self) -> Option<&str>;

    /// 设置 Condition 级线程池构建器名称。
    ///
    /// - `thread_pool_executor_class`: Java 构建器类名或 Rust 注册键。
    ///
    /// 对应 Java: `LoopCondition#setThreadPoolExecutorClass`。
    fn set_thread_pool_executor_class(&mut self, thread_pool_executor_class: impl Into<String>)
    where
        Self: Sized;

    /// 返回循环是否采用并行执行。对应 Java: `LoopCondition#isParallel`。
    fn is_parallel(&self) -> bool;

    /// 设置循环是否采用并行执行。
    ///
    /// 关闭时清除并行标记；开启时保留已有并行参数，没有参数则写入零值标记。
    /// 对应 Java: `LoopCondition#setParallel`。
    fn set_parallel(&mut self, parallel: bool);

    /// 构造一轮并行循环任务。
    ///
    /// Rust 以克隆后的 `Ctx/Frame` 对应 Java 的 `currChainId/slotIndex`，
    /// `it_obj` 对应 IteratorCondition 的当前循环对象。返回对象的 `get` 会在
    /// 指定执行器中运行真实循环体。对应 Java:
    /// `LoopCondition#LoopParallelSupplier`。
    fn loop_parallel_supplier(
        &self,
        executable_item: Arc<dyn Executable>,
        ctx: &Ctx,
        frame: &Frame,
        loop_index: usize,
        it_obj: Option<Value>,
        executor_service: Arc<ExecutorService>,
    ) -> LoopParallelSupplier {
        LoopParallelSupplier::new(
            executable_item,
            ctx.clone(),
            frame.clone(),
            loop_index,
            it_obj,
            executor_service,
        )
    }
}

/// 并行循环单轮任务封装。
///
/// 这是 Java `LoopCondition.LoopParallelSupplier` 内部类的 Rust 伴随类型，因此
/// 与主对象保留在同一文件。它持有隔离的执行上下文，不借用调用线程的临时状态。
pub struct LoopParallelSupplier {
    executable_item: Arc<dyn Executable>,
    ctx: Ctx,
    frame: Frame,
    loop_index: usize,
    it_obj: Option<Value>,
    executor_service: Arc<ExecutorService>,
}

impl LoopParallelSupplier {
    fn new(
        executable_item: Arc<dyn Executable>,
        ctx: Ctx,
        frame: Frame,
        loop_index: usize,
        it_obj: Option<Value>,
        executor_service: Arc<ExecutorService>,
    ) -> Self {
        Self {
            executable_item,
            ctx,
            frame,
            loop_index,
            it_obj,
            executor_service,
        }
    }

    /// 执行一轮循环并封装成功或失败结果。
    ///
    /// 执行前把循环下标和可选迭代对象压入独立 Frame；错误保留为
    /// `LoopFutureObj`，由 `handle_future_list` 统一结算。对应 Java:
    /// `LoopCondition.LoopParallelSupplier#get`。
    pub async fn get(self) -> LoopFutureObj {
        let executor_name = self.executable_item.id().to_string();
        let frame = self.frame.push(self.loop_index, self.it_obj);
        let result = self
            .executor_service
            .execute(async { self.executable_item.execute(&self.ctx, &frame).await })
            .await
            .and_then(|result| result);
        match result {
            Ok(_) => LoopFutureObj::success(executor_name),
            Err(error) => LoopFutureObj::fail(executor_name, error),
        }
    }
}

/// 并行循环：提交一轮迭代任务，然后（对齐 Java）在启动线程检查 BREAK。
/// 返回 false 表示 BREAK，停止后续提交。
pub async fn submit_iteration<C>(
    loop_condition: &C,
    set: &mut JoinSet<LoopFutureObj>,
    body: &Arc<dyn Executable>,
    brk: Option<&Arc<dyn Executable>>,
    ctx: &Ctx,
    frame: &Frame,
    index: usize,
    object: Option<Value>,
    executor_service: &Arc<ExecutorService>,
) -> LFResult<bool>
where
    C: LoopCondition + ?Sized,
{
    // Java 由内部 Supplier 保存本轮上下文；Rust 同样先构造拥有所有权的任务对象，
    // 再交给 JoinSet，确保循环下标和对象不会被后续迭代覆盖。
    let supplier = loop_condition.loop_parallel_supplier(
        Arc::clone(body),
        ctx,
        frame,
        index,
        object.clone(),
        Arc::clone(executor_service),
    );
    set.spawn(supplier.get());

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
