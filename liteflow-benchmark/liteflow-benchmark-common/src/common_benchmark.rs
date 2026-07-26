//! 普通链路执行基准。

use std::time::Instant;

use liteflow_core::{FlowBus, cmp};
use serde_json::{Value, json};

use crate::BenchmarkReport;

/// 普通 THEN 链路的重复执行场景。
///
/// 对应 Java: `com.yomahub.liteflow.benchmark.CommonBenchmark`。
pub struct CommonBenchmark;

impl CommonBenchmark {
    /// 构建一次链路并执行指定次数。
    pub async fn run(iterations: usize) -> BenchmarkReport {
        let bus = FlowBus::new();
        bus.register(
            "benchmark_common",
            cmp(|ctx| async move {
                ctx.set_data("benchmark_common", json!(true));
                Ok(Value::Null)
            }),
        );
        bus.add_chain("benchmark_common_chain", "THEN(benchmark_common)")
            .expect("benchmark chain should build");
        let started = Instant::now();
        for _ in 0..iterations {
            let response = bus.execute("benchmark_common_chain").await;
            assert!(response.is_success(), "{}", response.message);
        }
        BenchmarkReport::new(iterations, started.elapsed())
    }
}
