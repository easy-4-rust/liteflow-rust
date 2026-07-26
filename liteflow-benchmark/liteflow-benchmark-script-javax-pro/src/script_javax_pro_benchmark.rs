//! JSR-223 Pro/GraalJS 生态位的 Rust 基准。

use std::time::Instant;

use liteflow_benchmark_common::BenchmarkReport;
use liteflow_core::FlowBus;
use liteflow_core::script::ScriptKind;
use liteflow_script_graaljs::GraalJavaScriptExecutor;
use serde_json::json;

/// 通过隔离的 Boa 运行时执行 GraalJS 兼容入口的重复场景。
///
/// 对应 Java: `ScriptJavaxProBenchmark`，不宣称支持 GraalVM 宿主互操作。
pub struct ScriptJavaxProBenchmark;

impl ScriptJavaxProBenchmark {
    /// 注册 GraalJS 兼容执行器并执行指定次数。
    pub async fn run(iterations: usize) -> BenchmarkReport {
        GraalJavaScriptExecutor::register().expect("GraalJS executor should register");
        let bus = FlowBus::new();
        bus.register_script_typed(
            "javax_pro_benchmark",
            "graaljs",
            ScriptKind::Common,
            "return input.value + 1;",
        )
        .expect("Javax Pro benchmark component should build");
        bus.add_chain("javax_pro_benchmark_chain", "THEN(javax_pro_benchmark)")
            .expect("Javax Pro benchmark chain should build");
        let started = Instant::now();
        for _ in 0..iterations {
            let response = bus
                .execute_with_data("javax_pro_benchmark_chain", json!({"value": 1}))
                .await;
            assert!(response.is_success(), "{}", response.message);
        }
        BenchmarkReport::new(iterations, started.elapsed())
    }
}
