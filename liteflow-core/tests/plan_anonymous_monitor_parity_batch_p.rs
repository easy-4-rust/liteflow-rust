//! ParserHelper 构建与匿名链/监控路径补测（批次 P）。
//!
//! 覆盖：
//! - `RuleDefinitionPlan#buildAll/buildChain` 的真实节点与链构建
//! - `FlowBus#addChainAnonymous/getChainIdByElMd5` 匿名链注册与查询
//! - `MonitorFile#onFileCreate` 与真实 reload 路径

use liteflow_core::parser::helper::{ChainDef, RuleDefinitionPlan};
use liteflow_core::{FlowBus, NodePropBean, cmp};
use md5::{Digest, Md5};
use serde_json::Value;
use std::time::Duration;

/// RuleDefinitionPlan 的两阶段构建：节点 + 链全部注册。
#[test]
fn plan_build_all_registers_nodes_and_chains() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    let mut plan = RuleDefinitionPlan::new();

    let mut node = NodePropBean::default().set_id("script_plan");
    node.node_type = Some("script".to_string());
    node.script = Some("let x = 1;".to_string());
    node.language = Some("rhai".to_string());
    plan.push_node(node);

    plan.push_chain(ChainDef {
        id: "plan_chain".to_string(),
        namespace: String::new(),
        route: None,
        body: "THEN(a)".to_string(),
        extends: None,
        thread_pool_executor_class: None,
        enable: true,
    });

    let built = plan.build_all(&bus).expect("计划应构建成功");
    assert_eq!(built, vec!["plan_chain".to_string()]);
    assert!(bus.contains_chain("plan_chain"));
    assert!(bus.contains_node("script_plan"));
}

/// build_chain 单独构建指定链（ParseOne 语义）。
#[test]
fn plan_build_chain_single() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    let mut plan = RuleDefinitionPlan::new();
    plan.push_chain(ChainDef {
        id: "single_chain".to_string(),
        namespace: String::new(),
        route: None,
        body: "THEN(a)".to_string(),
        extends: None,
        thread_pool_executor_class: None,
        enable: true,
    });
    plan.build_chain(&bus, "single_chain")
        .expect("单链构建成功");
    assert!(bus.contains_chain("single_chain"));
    // 已存在链幂等
    plan.build_chain(&bus, "single_chain").expect("幂等");
}

/// 匿名链注册与 MD5 查询。
#[tokio::test]
async fn anonymous_chain_registration_and_md5_query() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    let normalized = liteflow_core::util::el_regex_util::ElRegexUtil::normalize("THEN(a)");
    let md5 = format!("{:x}", Md5::digest(normalized.as_bytes()));
    bus.add_chain_anonymous("anon-chain", &normalized, md5.clone())
        .expect("匿名链注册");
    let chain_id = bus.get_chain_id_by_el_md5(&md5);
    assert!(chain_id.is_some());
    // 执行匿名链成功
    let executor = liteflow_core::FlowExecutor::new_isolated(
        bus.clone(),
        liteflow_core::LiteflowConfig::default(),
    );
    let response = executor.execute(chain_id.as_deref().unwrap()).await;
    assert!(response.is_success());
}

/// MonitorFile 文件创建事件与重载。
#[tokio::test]
async fn monitor_file_create_and_reload() {
    let dir = std::env::temp_dir().join("liteflow-monitor-p");
    std::fs::create_dir_all(&dir).unwrap();
    let rule_file = dir.join("flow.xml");
    std::fs::write(&rule_file, "<flow/>").unwrap();

    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    let monitor = liteflow_core::monitor::MonitorFile::new(bus.clone());
    monitor.add_monitor_file_path(&rule_file).expect("登记");
    monitor.create(Duration::from_millis(300)).expect("启动");

    // 创建事件（文件已存在时按创建处理）
    std::fs::write(&rule_file, "<flow/>").unwrap();
    monitor.on_file_create(&rule_file).expect("创建事件");

    // 真实规则文件重载
    std::fs::write(
        &rule_file,
        r#"<flow><chain id="monitor_chain">THEN(a)</chain></flow>"#,
    )
    .unwrap();
    monitor.on_file_change(&rule_file).expect("重载");
    assert!(bus.contains_chain("monitor_chain"));

    monitor.destroy().expect("销毁");
    let _ = std::fs::remove_dir_all(&dir);
}

/// FlowBus 的节点存在性与 Chain 查询 API。
#[test]
fn flow_bus_node_and_chain_queries() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    bus.add_chain("q_chain", "THEN(a)").unwrap();
    assert!(bus.contains_node("a"));
    assert!(!bus.contains_node("missing"));
    assert!(bus.contains_chain("q_chain"));
    assert!(!bus.contains_chain("missing"));
}
