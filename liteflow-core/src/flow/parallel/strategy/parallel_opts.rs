//! 并行等待公共选项。

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use crate::thread::ExecutorService;

/// 对应 Java `WhenCondition` 的并行等待字段。
pub struct ParallelOpts {
    /// 是否忽略普通分支错误。
    pub ignore_error: bool,
    /// 必须成功的分支序号。
    pub must_idx: HashSet<usize>,
    /// 需要完成的分支比例。
    ///
    /// 对应 Java: `WhenCondition#getPercentage`。非 percentage 策略忽略此值。
    pub percentage: Option<f64>,
    /// 每个并行分支各自的最大等待时间。
    ///
    /// Java 在 `ParallelStrategyExecutor#wrappedFutureObj` 中对每个
    /// `CompletableFuture` 独立调用 `completeOnTimeout`；Rust 在分支包装任务中
    /// 使用同一时长生成 `WhenFutureObj::time_out`。
    pub max_wait: Duration,
    /// 本次 WHEN 分支实际使用的有界执行器。
    ///
    /// 对应 Java `ExecutorHelper#buildExecutorService` 的返回值。
    pub executor_service: Arc<ExecutorService>,
}
