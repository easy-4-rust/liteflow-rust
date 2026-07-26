//! 所有基准场景的最小真实执行门禁。

#[tokio::test]
async fn every_benchmark_scenario_executes_real_work() {
    let reports = [
        liteflow_benchmark::common::CommonBenchmark::run(2).await,
        liteflow_benchmark::common_example::CommonExampleBenchmark::run(2).await,
        liteflow_benchmark::compile::CompileBenchmark::run(2),
        liteflow_benchmark::script_groovy::ScriptGroovyBenchmark::run(2).await,
        liteflow_benchmark::script_java::ScriptJavaBenchmark::run(2).await,
        liteflow_benchmark::script_javax::ScriptJavaxBenchmark::run(2).await,
        liteflow_benchmark::script_javax_pro::ScriptJavaxProBenchmark::run(2).await,
        liteflow_benchmark::script_qlexpress::ScriptQlExpressBenchmark::run(2).await,
    ];
    assert!(reports.iter().all(|report| report.iterations() == 2));
    assert!(reports.iter().all(|report| report.elapsed().as_nanos() > 0));
}
