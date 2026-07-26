//! EL 编译基准。

use std::time::Instant;

use liteflow_benchmark_common::BenchmarkReport;
use liteflow_core::parse_el;

/// 复杂 LiteFlow EL 的重复解析场景。
///
/// 对应 Java `liteflow-benchmark-compile` 模块。
pub struct CompileBenchmark;

impl CompileBenchmark {
    /// 重复解析包含 WHEN、IF 与 FOR 的表达式。
    pub fn run(iterations: usize) -> BenchmarkReport {
        let started = Instant::now();
        for _ in 0..iterations {
            parse_el("THEN(a, WHEN(b, c), IF(d, e, f), FOR(3).DO(g))")
                .expect("benchmark EL should compile");
        }
        BenchmarkReport::new(iterations, started.elapsed())
    }
}
