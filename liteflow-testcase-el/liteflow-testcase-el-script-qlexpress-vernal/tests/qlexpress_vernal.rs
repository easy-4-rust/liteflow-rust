//! QLExpress 发布版运行时与 Vernal 组合场景的独立真实门禁。

/// 验证 testcase 入口完成 QLExpress 注册、脚本节点构建、FlowBus 执行与
/// DefaultContext 写回，而不依赖聚合测试中的其他外部规则服务。
///
/// 对应 Java: `QLExpressScriptExecutor` 与 LiteFlow 容器启动组合测试。
#[tokio::test]
async fn published_qlexpress_executes_in_vernal_combination() {
    assert!(
        liteflow_testcase_el_script_qlexpress_vernal::run_case().await,
        "QlExpress Rust 发布版运行时应通过 Vernal 组合入口完成真实流程执行"
    );
}
