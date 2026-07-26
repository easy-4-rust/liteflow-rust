//! JSR-223 JavaScript 生态位的 Rust 基准。

use std::time::Instant;

use liteflow_benchmark_common::BenchmarkReport;
use liteflow_core::FlowBus;
use liteflow_core::script::ScriptKind;
use liteflow_script_javascript::JavaScriptExecutor;
use serde_json::json;

/// 通过 Boa ECMAScript 运行时执行 JavaScript 的重复场景。
///
/// 对应 Java: `ScriptJavaxBenchmark`。
pub struct ScriptJavaxBenchmark;

impl ScriptJavaxBenchmark {
    /// 注册 JavaScript 执行器并执行指定次数。
    pub async fn run(iterations: usize) -> BenchmarkReport {
        JavaScriptExecutor::register().expect("JavaScript executor should register");
        let bus = FlowBus::new();
        bus.register_script_typed(
            "javax_benchmark",
            "javascript",
            ScriptKind::Common,
            "return input.value + 1;",
        )
        .expect("Javax benchmark component should build");
        bus.add_chain("javax_benchmark_chain", "THEN(javax_benchmark)")
            .expect("Javax benchmark chain should build");
        let started = Instant::now();
        for _ in 0..iterations {
            let response = bus
                .execute_with_data("javax_benchmark_chain", json!({"value": 1}))
                .await;
            assert!(response.is_success(), "{}", response.message);
        }
        BenchmarkReport::new(iterations, started.elapsed())
    }
}
