//! 基准执行结果。

use std::time::Duration;

/// 一次 Rust 基准场景的迭代数与总耗时。
///
/// 对应 JMH `RunResult` 在本工程内需要保留的最小统计维度。
#[derive(Debug, Clone, Copy)]
pub struct BenchmarkReport {
    iterations: usize,
    elapsed: Duration,
}

impl BenchmarkReport {
    /// 创建基准结果。
    #[must_use]
    pub fn new(iterations: usize, elapsed: Duration) -> Self {
        Self {
            iterations,
            elapsed,
        }
    }

    /// 返回已完成迭代数。
    #[must_use]
    pub fn iterations(&self) -> usize {
        self.iterations
    }

    /// 返回场景总耗时。
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// 返回每秒完成的迭代数。
    #[must_use]
    pub fn operations_per_second(&self) -> f64 {
        self.iterations as f64 / self.elapsed.as_secs_f64().max(f64::EPSILON)
    }
}
