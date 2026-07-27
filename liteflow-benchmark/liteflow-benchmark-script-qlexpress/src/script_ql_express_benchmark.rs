//! QLExpress 独立解析器与解释器基准。

use std::time::Instant;

use liteflow_benchmark_common::BenchmarkReport;
use liteflow_core::FlowBus;
use liteflow_core::script::ScriptKind;
use liteflow_script_qlexpress::QlExpressScriptExecutor;

/// 使用 Java QLExpress 语句语法重复执行 Rust 原生 QLExpress 解释器。
///
/// 对应 Java: `ScriptQLExpressBenchmark`。
pub struct ScriptQlExpressBenchmark;

impl ScriptQlExpressBenchmark {
    /// 注册 QLExpress 执行器并执行指定次数，返回耗时报告。
    ///
    /// `iterations` 表示链路执行次数。对应 Java:
    /// `ScriptQLExpressBenchmark` 的重复脚本执行场景。
    pub async fn run(iterations: usize) -> BenchmarkReport {
        QlExpressScriptExecutor::register().expect("QLExpress executor should register");
        let bus = FlowBus::new();
        bus.register_script_typed(
            "qlexpress_benchmark",
            "qlexpress",
            ScriptKind::Common,
            "a = 1; b = 1; defaultContext.setData(\"benchmark\", a + b);",
        )
        .expect("QLExpress benchmark component should build");
        bus.add_chain("qlexpress_benchmark_chain", "THEN(qlexpress_benchmark)")
            .expect("QLExpress benchmark chain should build");
        let started = Instant::now();
        for _ in 0..iterations {
            let response = bus.execute("qlexpress_benchmark_chain").await;
            assert!(response.is_success(), "{}", response.message);
            assert_eq!(response.data_as::<i64>("benchmark"), Some(2));
        }
        BenchmarkReport::new(iterations, started.elapsed())
    }
}
