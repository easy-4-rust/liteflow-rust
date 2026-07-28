//! Kotlin Java testcase 与 Vernal 组合入口的独立真实门禁。

/// 验证 Kotlin 执行器注册、函数编译、bindings 上下文和 FlowBus 循环执行。
///
/// 对应 Java: `LiteFlowKotlinScriptCommonELTest`。
#[tokio::test]
async fn kotlin_java_baseline_executes_in_vernal_combination() {
    assert!(
        liteflow_testcase_el_script_kotlin_vernal::run_case().await,
        "Kotlin Java testcase 基线应通过 Vernal 组合入口完成真实流程执行"
    );
}
