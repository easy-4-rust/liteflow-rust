//! QLExpress 公共表达式子集基准。

use std::time::Instant;

use liteflow_benchmark_common::BenchmarkReport;
use liteflow_core::FlowBus;
use liteflow_core::script::ScriptKind;
use liteflow_script_qlexpress::QlExpressScriptExecutor;
use serde_json::json;

/// QLExpress 生态位映射到 Rust 原生 Rhai 子集的重复执行场景。
///
/// 对应 Java: `ScriptQLExpressBenchmark`。
pub struct ScriptQlExpressBenchmark;

impl ScriptQlExpressBenchmark {
    /// 注册 QLExpress 执行器并执行指定次数。
    pub async fn run(iterations: usize) -> BenchmarkReport {
        QlExpressScriptExecutor::register().expect("QLExpress executor should register");
        let bus = FlowBus::new();
        bus.register_script_typed(
            "qlexpress_benchmark",
            "qlexpress",
            ScriptKind::Common,
            "return input.value + 1;",
        )
        .expect("QLExpress benchmark component should build");
        bus.add_chain("qlexpress_benchmark_chain", "THEN(qlexpress_benchmark)")
            .expect("QLExpress benchmark chain should build");
        let started = Instant::now();
        for _ in 0..iterations {
            let response = bus
                .execute_with_data("qlexpress_benchmark_chain", json!({"value": 1}))
                .await;
            assert!(response.is_success(), "{}", response.message);
        }
        BenchmarkReport::new(iterations, started.elapsed())
    }
}
