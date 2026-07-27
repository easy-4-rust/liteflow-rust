//! Groovy LiteFlow 上下文绑定执行基准。

use std::time::Instant;

use liteflow_benchmark_common::BenchmarkReport;
use liteflow_core::FlowBus;
use liteflow_core::script::ScriptKind;
use liteflow_script_groovy::GroovyScriptExecutor;

/// 使用 Java Groovy 常用语法重复执行 Rust 受控适配器。
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
            "def a = 1\ndef b = 1\ndefaultContext.setData(\"benchmark\", a + b)",
        )
        .expect("Groovy benchmark component should build");
        bus.add_chain("groovy_benchmark_chain", "THEN(groovy_benchmark)")
            .expect("Groovy benchmark chain should build");
        let started = Instant::now();
        for _ in 0..iterations {
            let response = bus.execute("groovy_benchmark_chain").await;
            assert!(response.is_success(), "{}", response.message);
            assert_eq!(response.data_as::<i64>("benchmark"), Some(2));
        }
        BenchmarkReport::new(iterations, started.elapsed())
    }
}
