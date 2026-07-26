//! Java 编译脚本生态位的 Rust 映射基准。

use std::time::Instant;

use liteflow_benchmark_common::BenchmarkReport;
use liteflow_core::FlowBus;
use liteflow_core::script::ScriptKind;
use serde_json::json;

/// 使用 core 内建 Rhai 编译执行替代 JVM 动态 Java 编译的场景。
///
/// 对应 Java: `ScriptJavaBenchmark`；Rust 不嵌入 JVM，因此测量相同的
/// “构建期编译、执行期复用”职责边界。
pub struct ScriptJavaBenchmark;

impl ScriptJavaBenchmark {
    /// 构建一次脚本组件并执行指定次数。
    pub async fn run(iterations: usize) -> BenchmarkReport {
        let bus = FlowBus::new();
        bus.register_script_typed(
            "java_benchmark",
            "rhai",
            ScriptKind::Common,
            "input.value + 1",
        )
        .expect("Java-mapped benchmark component should build");
        bus.add_chain("java_benchmark_chain", "THEN(java_benchmark)")
            .expect("Java-mapped benchmark chain should build");
        let started = Instant::now();
        for _ in 0..iterations {
            let response = bus
                .execute_with_data("java_benchmark_chain", json!({"value": 1}))
                .await;
            assert!(response.is_success(), "{}", response.message);
        }
        BenchmarkReport::new(iterations, started.elapsed())
    }
}
