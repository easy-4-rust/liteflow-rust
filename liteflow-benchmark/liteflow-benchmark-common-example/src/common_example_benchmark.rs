//! 示例订单链路执行基准。

use std::time::Instant;

use liteflow_benchmark_common::BenchmarkReport;
use liteflow_core::{FlowBus, cmp};
use serde_json::{Value, json};

/// 多节点示例链路的重复执行场景。
///
/// 对应 Java: `com.yomahub.liteflow.benchmark.CommonExampleBenchmark`。
pub struct CommonExampleBenchmark;

impl CommonExampleBenchmark {
    /// 执行包含串行与条件分支的示例链路。
    pub async fn run(iterations: usize) -> BenchmarkReport {
        let bus = FlowBus::new();
        bus.register("prepare", cmp(|_| async { Ok(Value::Null) }));
        bus.register("allowed", cmp(|_| async { Ok(Value::Bool(true)) }));
        bus.register(
            "finish",
            cmp(|ctx| async move {
                ctx.set_data("finished", json!(true));
                Ok(Value::Null)
            }),
        );
        bus.add_chain(
            "benchmark_example_chain",
            "THEN(prepare, IF(allowed, finish))",
        )
        .expect("example benchmark chain should build");
        let started = Instant::now();
        for _ in 0..iterations {
            let response = bus.execute("benchmark_example_chain").await;
            assert!(response.is_success(), "{}", response.message);
        }
        BenchmarkReport::new(iterations, started.elapsed())
    }
}
