//! Groovy 公共表达式子集执行基准。

use std::time::Instant;

use liteflow_benchmark_common::BenchmarkReport;
use liteflow_core::FlowBus;
use liteflow_core::script::ScriptKind;
use liteflow_script_groovy::GroovyScriptExecutor;
use serde_json::json;

/// Groovy 生态位映射到 Rust 原生 Rhai 子集的重复执行场景。
///
/// 对应 Java: `ScriptGroovyBenchmark`。
pub struct ScriptGroovyBenchmark;

impl ScriptGroovyBenchmark {
    /// 注册 Groovy 执行器并执行指定次数。
    pub async fn run(iterations: usize) -> BenchmarkReport {
        GroovyScriptExecutor::register().expect("Groovy executor should register");
        let bus = FlowBus::new();
        bus.register_script_typed(
            "groovy_benchmark",
            "groovy",
            ScriptKind::Common,
            "return input.value + 1;",
        )
        .expect("Groovy benchmark component should build");
        bus.add_chain("groovy_benchmark_chain", "THEN(groovy_benchmark)")
            .expect("Groovy benchmark chain should build");
        let started = Instant::now();
        for _ in 0..iterations {
            let response = bus
                .execute_with_data("groovy_benchmark_chain", json!({"value": 1}))
                .await;
            assert!(response.is_success(), "{}", response.message);
        }
        BenchmarkReport::new(iterations, started.elapsed())
    }
}
