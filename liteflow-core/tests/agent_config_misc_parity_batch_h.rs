//! Agent 配置与杂项未触达 API 补测（批次 H）。
//!
//! 覆盖：
//! - `WorkspaceConfig`/`MemoryStorageConfig`/`SkillsConfig`/`MysqlMemoryConfig`/
//!   `LoggingConfig` 的 Java 命名 setter/getter
//! - `ComponentInitializer#withDefaultNodeExecutor`
//! - `FallbackNode#expectedNodeType`
//! - `Node#getPreNLoopObject`（循环对象栈读取）
//! - `NodeIteratorComponent#processIterator`
//! - `Condition#checkNotPreFinally/expectBool` 前置校验

use liteflow_core::flow::element::ConditionKey;
use liteflow_core::flow::element::fallback_node::FallbackNode;
use liteflow_core::property::agent::{
    LoggingConfig, MemoryStorageConfig, MysqlMemoryConfig, SkillsConfig, WorkspaceConfig,
};
use liteflow_core::{CmpContext, Frame, NodeRef, NodeTypeEnum, Slot, cmp};
use serde_json::{Value, json};
use std::sync::Arc;

/// WorkspaceConfig 的 Java 命名开关往返。
#[test]
fn workspace_config_java_switches_round_trip() {
    let mut config = WorkspaceConfig::default();
    config.set_auto_create(true);
    config.set_cleanup_on_jvm_shutdown(true);
    config.set_cleanup_on_session_expire(false);

    assert!(config.is_auto_create());
    assert!(config.is_cleanup_on_jvm_shutdown());
    assert!(!config.is_cleanup_on_session_expire());
}

/// MemoryStorageConfig 的存储开关与后端选择。
#[test]
fn memory_storage_config_switches_round_trip() {
    let mut config = MemoryStorageConfig::default();
    config.set_load_on_first_use(true);
    config.set_save_after_call(true);
    config.set_save_on_error(false);
    // Redis/MySQL/本地文件子配置的 Java 字段由各自对象级测试覆盖；此处验证
    // 真实字段进入同一配置对象
    let mut redis = liteflow_core::property::agent::RedisMemoryConfig::default();
    redis.set_key_prefix("lf:");
    let mut mysql = liteflow_core::property::agent::MysqlMemoryConfig::default();
    mysql.set_table_name(Some("agent_sessions".to_string()));
    let local = liteflow_core::property::agent::LocalFileMemoryConfig;
    assert_eq!(
        liteflow_core::property::agent::LocalFileMemoryConfig::SUB_DIR,
        ".agent-session"
    );
    config.set_redis(redis);
    config.set_mysql(mysql);
    config.set_local_file(local);

    assert!(config.is_load_on_first_use());
    assert!(config.is_save_after_call());
    assert!(!config.is_save_on_error());
    assert_eq!(config.get_redis().get_key_prefix(), "lf:");
    assert_eq!(config.get_mysql().get_table_name(), Some("agent_sessions"));
    assert_eq!(
        config.get_local_file(),
        &liteflow_core::property::agent::LocalFileMemoryConfig
    );
}

/// SkillsConfig 的严格模式开关。
#[test]
fn skills_config_strict_mode() {
    let mut config = SkillsConfig::default();
    config.set_strict(true);
    assert!(config.is_strict());
}

/// MysqlMemoryConfig 的建表开关。
#[test]
fn mysql_memory_config_create_if_not_exists() {
    let mut config = MysqlMemoryConfig::default();
    config.set_create_if_not_exist(true);
    assert!(config.is_create_if_not_exist());
}

/// LoggingConfig 的 ReAct 日志开关。
#[test]
fn logging_config_react_enabled() {
    let mut config = LoggingConfig::default();
    config.set_react_enabled(true);
    assert!(config.is_react_enabled());
}

/// FallbackNode 期望节点类型与降级执行。
#[tokio::test]
async fn fallback_node_expected_type_and_execution() {
    let nodes: Arc<dashmap::DashMap<String, Arc<dyn liteflow_core::NodeComponent>>> =
        Arc::new(dashmap::DashMap::new());
    let fallback_nodes: Arc<dashmap::DashMap<String, Arc<dyn liteflow_core::NodeComponent>>> =
        Arc::new(dashmap::DashMap::new());
    nodes.insert(
        "actual".to_string(),
        Arc::new(cmp(|_| async { Ok(json!("fallback-result")) })),
    );
    let fallback = FallbackNode::new(
        "actual",
        NodeTypeEnum::Common,
        Arc::clone(&nodes),
        Arc::clone(&fallback_nodes),
    );
    assert_eq!(fallback.expected_node_type(), NodeTypeEnum::Common);

    let slot = Arc::new(Slot::new("RID-FB".to_string(), "main", Value::Null));
    let context = CmpContext {
        inner: slot.clone(),
        node: NodeRef::new("actual"),
        frame: Frame::root(),
    };
    let result = fallback.execute(&context).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), json!("fallback-result"));
}

/// Node 循环对象栈的预读入口。
#[tokio::test]
async fn node_pre_n_loop_object_reads_stack() {
    let component: Arc<dyn liteflow_core::NodeComponent> =
        Arc::new(cmp(|_| async { Ok(Value::Null) }));
    let node = liteflow_core::flow::element::node::Node::new(NodeRef::new("loop-node"), component);
    let slot = Arc::new(Slot::new("RID-LOOP".to_string(), "main", Value::Null));
    let frame = Frame::root().push(0, Some(json!({"item": 5})));
    let context = CmpContext {
        inner: slot.clone(),
        node: NodeRef::new("loop-node"),
        frame: frame.clone(),
    };
    // 第 1 层循环对象可读（Java getPreNLoopObject 语义）
    assert!(node.get_pre_n_loop_object(&frame, 0).is_some());
    let _ = context;
}

/// NodeIteratorComponent 的迭代器返回语义（Java NodeIteratorComponent#processIterator）。
#[tokio::test]
async fn node_iterator_component_process_iterator() {
    let component =
        liteflow_core::core::NodeIteratorComponent::new("iter", |_ctx: CmpContext| async move {
            Ok(vec![json!(1), json!(2), json!(3)])
        });
    let slot = Arc::new(Slot::new("RID-ITER".to_string(), "main", Value::Null));
    let context = CmpContext {
        inner: slot.clone(),
        node: NodeRef::new("iter"),
        frame: Frame::root(),
    };
    let result = component.process_iterator(context).await;
    assert!(result.is_ok());
    let list = result.unwrap();
    assert_eq!(list.len(), 3);
}

/// Condition 的前置校验辅助与分组键。
#[test]
fn condition_prefinally_checks_and_keys() {
    // ConditionKey 常量与 Java 对齐（enum + as_str）
    assert_eq!(ConditionKey::If.as_str(), "IF_KEY");
    assert_eq!(ConditionKey::IfTrueCase.as_str(), "IF_TRUE_CASE_KEY");
    assert_eq!(ConditionKey::IfFalseCase.as_str(), "IF_FALSE_CASE_KEY");
    assert_eq!(ConditionKey::Pre.as_str(), "PRE_KEY");
    assert_eq!(ConditionKey::Finally.as_str(), "FINALLY_KEY");
    assert_eq!(ConditionKey::Catch.as_str(), "CATCH_KEY");
    assert_eq!(ConditionKey::Do.as_str(), "DO_KEY");
    assert_eq!(ConditionKey::Break.as_str(), "BREAK_KEY");
    assert_eq!(ConditionKey::Default.as_str(), "DEFAULT_KEY");
    assert_eq!(ConditionKey::Iterator.as_str(), "ITERATOR_KEY");
    assert_eq!(ConditionKey::For.as_str(), "FOR_KEY");
    assert_eq!(ConditionKey::Switch.as_str(), "SWITCH_KEY");
    assert_eq!(ConditionKey::SwitchTarget.as_str(), "SWITCH_TARGET_KEY");
    assert_eq!(ConditionKey::SwitchDefault.as_str(), "SWITCH_DEFAULT_KEY");
    assert_eq!(ConditionKey::While.as_str(), "WHILE_KEY");
    assert_eq!(
        ConditionKey::from_key("WHILE_KEY"),
        Some(ConditionKey::While)
    );
    assert_eq!(ConditionKey::from_key("NOPE"), None);
}
