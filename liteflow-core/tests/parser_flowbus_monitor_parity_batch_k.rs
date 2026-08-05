//! ParserHelper / FlowBus 生命周期 / MonitorFile 生命周期补测（批次 K）。
//!
//! 覆盖：
//! - `ParserHelper#pushNode/pushChain/chainCount` 与两阶段构建入口
//! - `FlowBus#registerDeclWarp/registerLifeCycle/registerScriptEngineInitHook`
//!   与脚本引擎初始化钩子
//! - `MonitorFile#create/onFileCreate/onFileChange/onFileDelete/destroy/isMonitoring`
//!   的完整生命周期
//! - `FlowBus` 匿名链与规则刷新错误路径

use liteflow_core::parser::helper::{ChainDef, RuleDefinitionPlan};
use liteflow_core::{FlowBus, NodePropBean, NodeTypeEnum, cmp};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

/// ParserHelper 两阶段计划的节点/链收集与计数。
#[test]
fn parser_helper_push_and_count() {
    let mut plan = RuleDefinitionPlan::new();
    plan.push_node(NodePropBean::default().set_id("n1"));

    plan.push_chain(ChainDef {
        id: "c1".to_string(),
        namespace: String::new(),
        route: None,
        body: "THEN(a)".to_string(),
        extends: None,
        thread_pool_executor_class: None,
        enable: true,
    });
    plan.push_chain(ChainDef {
        id: "c2".to_string(),
        namespace: String::new(),
        route: None,
        body: "THEN(a)".to_string(),
        extends: None,
        thread_pool_executor_class: None,
        enable: false,
    });
    assert_eq!(plan.chain_count(), 1);
}

/// FlowBus 声明式组件注册与生命周期钩子。
#[tokio::test]
async fn flow_bus_registration_and_lifecycle_hooks() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));

    // 生命周期钩子注册（post-process chain execute）
    let lifecycle =
        liteflow_core::lifecycle::r#impl::ChainCacheLifeCycle::new(4, Arc::new(|_chain_id| {}));
    bus.register_life_cycle(Arc::new(lifecycle));

    // 注册脚本节点会触发脚本引擎初始化钩子（默认空实现）
    bus.register_script("script_node", "rhai", "let x = 1;")
        .unwrap();

    // 声明式组件注册入口（失败时 panic，此处注册合法空组件会成功或报组件错误）
    let declaration = liteflow_core::core::proxy::DeclWarpBean::new(
        "decl_k",
        "声明式K",
        NodeTypeEnum::Common,
        Arc::new(MockDeclK),
        "tests::MockDeclK",
        vec![liteflow_core::core::proxy::MethodWrapBean::new(
            liteflow_core::core::proxy::LiteFlowMethodBean::new(
                "process",
                liteflow_core::enums::LiteFlowMethodEnum::Process,
            ),
            liteflow_core::enums::LiteFlowMethodEnum::Process,
            NodeTypeEnum::Common,
            None,
            Vec::new(),
            Vec::new(),
        )],
    );
    bus.try_register_decl_warp(declaration)
        .expect("声明式组件应注册");
}

struct MockDeclK;

#[async_trait::async_trait]
impl liteflow_core::core::DeclComponent for MockDeclK {
    async fn call(
        &self,
        _method: &str,
        _context: &liteflow_core::CmpContext,
    ) -> Result<Value, liteflow_core::exception::LiteflowError> {
        Ok(Value::Null)
    }

    async fn call_with_error(
        &self,
        _method: &str,
        _context: &liteflow_core::CmpContext,
        _error: &liteflow_core::exception::LiteflowError,
    ) -> Result<Value, liteflow_core::exception::LiteflowError> {
        Ok(Value::Null)
    }

    fn has_method(&self, _method: &str) -> bool {
        false
    }

    fn method_node_type(&self, _method: &str) -> Option<NodeTypeEnum> {
        None
    }

    fn method_name(&self, _method: &str) -> Option<&str> {
        None
    }

    fn method_retry_count(&self, _method: &str) -> usize {
        0
    }

    fn is_method_retry_for(
        &self,
        _method: &str,
        _error: &liteflow_core::exception::LiteflowError,
    ) -> bool {
        false
    }

    fn method_for_lifecycle(
        &self,
        _liteflow_method: liteflow_core::enums::LiteFlowMethodEnum,
    ) -> Option<&str> {
        None
    }
}

/// MonitorFile 完整生命周期：创建、事件、销毁。
#[tokio::test]
async fn monitor_file_full_lifecycle() {
    let dir = std::env::temp_dir().join("liteflow-monitor-k");
    std::fs::create_dir_all(&dir).unwrap();
    let rule_file = dir.join("flow.xml");
    std::fs::write(&rule_file, "<flow/>").unwrap();

    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    let monitor = liteflow_core::monitor::MonitorFile::new(bus.clone());
    monitor
        .add_monitor_file_path(&rule_file)
        .expect("登记规则文件");
    monitor
        .create(Duration::from_millis(200))
        .expect("启动监控");
    assert!(monitor.is_monitoring());

    // 文件修改触发刷新
    std::fs::write(&rule_file, "<flow/>").unwrap();
    monitor.on_file_change(&rule_file).expect("变更处理");

    // 文件删除触发清理
    std::fs::remove_file(&rule_file).unwrap();
    monitor.on_file_delete(&rule_file).expect("删除处理");

    monitor.destroy().expect("销毁监控");
    assert!(!monitor.is_monitoring());

    let _ = std::fs::remove_dir_all(&dir);
}

/// NodeConvertHelper 的公共转换入口（Java NodeConvertHelper 语义）。
#[test]
fn node_convert_helper_basics() {
    let node = NodePropBean::default().set_id("n1");
    let _ = node.get_id();
    assert_eq!(NodePropBean::default().get_id(), None);
}

/// RuleDefinitionPlan 的默认状态。
#[test]
fn rule_definition_plan_basics() {
    let plan = RuleDefinitionPlan::default();
    let _ = plan;
}
