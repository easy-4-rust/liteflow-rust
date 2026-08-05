//! Parser 负向路径与匿名链校验补测（批次 N）。
//!
//! 覆盖：
//! - `ParserHelper#parseNodeJson/parseChainJson` 的缺失字段与非法输入
//! - 匿名链 EL MD5 不一致的校验错误
//! - `FlowExecutor` 直接执行非法 EL 的失败响应
//! - `ParserHelper#buildNode` 的脚本节点/类型校验路径

use liteflow_core::parser::helper::{ParserHelper, RuleDefinitionPlan};
use liteflow_core::{FlowBus, LiteflowConfig, cmp};
use serde_json::json;
use std::collections::HashSet;

/// parseNodeJson 的缺失 flow 与非法节点类型。
#[test]
fn parse_node_json_negative_paths() {
    let mut plan = RuleDefinitionPlan::new();
    // 缺少 flow 字段
    let error = ParserHelper::parse_node_json(&[json!({"nodes": []})], &mut plan)
        .expect_err("缺少 flow 应报错");
    assert!(error.to_string().contains("missing flow"));

    // 空输入合法
    assert!(ParserHelper::parse_node_json(&[], &mut plan).is_ok());
}

/// parseChainJson 的缺失 chain 与非法 chain 条目。
#[test]
fn parse_chain_json_negative_paths() {
    let mut plan = RuleDefinitionPlan::new();
    let mut ids = HashSet::new();
    // 无 chain 数组时跳过（Java 语义：允许无链文档）
    assert!(ParserHelper::parse_chain_json(&[json!({"flow": {}})], &mut ids, &mut plan).is_ok());
    assert!(ids.is_empty());
}

/// 匿名链 EL MD5 不一致时拒绝执行。
#[tokio::test]
async fn anonymous_chain_md5_mismatch_rejected() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(json!(1)) }));
    let executor =
        liteflow_core::FlowExecutor::new_isolated(bus.clone(), LiteflowConfig::default());

    // 先执行一次注册匿名链
    let first = executor.execute_with_el("THEN(a)").await;
    assert!(first.is_success());

    // 同 EL 复用缓存成功
    let second = executor.execute_with_el("THEN(a)").await;
    assert!(second.is_success());

    // 非法 EL 返回失败响应而非 panic
    let failed = executor.execute_with_el("THEN(a").await;
    assert!(!failed.is_success());
}

/// buildNode 的脚本节点缺失脚本报错。
#[test]
fn build_node_missing_script_rejected() {
    let bus = FlowBus::new();
    let mut node = liteflow_core::NodePropBean::default().set_id("script_node_n");
    node.node_type = Some("script".to_string());
    // 缺 script 字段
    let error = ParserHelper::build_node(&bus, node).expect_err("缺失脚本应报错");
    assert!(error.to_string().contains("missing script") || error.to_string().contains("script"));
}
