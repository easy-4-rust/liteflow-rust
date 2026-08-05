//! FlowExecutor / ExecuteOption / 脚本 Bean / 实例编号域的补测（批次 F）。
//!
//! 覆盖：
//! - `FlowExecutor#execute2RespWithRid/execute2RespWithEL 全参入口`
//! - `ExecuteOption#genConversationId`
//! - `ScriptBeanManager#addScriptBean/getScriptBean/getScriptBeanMap/removeScriptBean`
//! - `JsonConvert#jsonToDynamic/dynamicToJson`
//! - `BaseNodeInstanceIdManageSpi#assignInstanceIds/restoreInstanceIds`
//! - `ScriptExecutorComponent#executor`
//! - `NodeComponent#processIterator`（脚本迭代器组件）

use liteflow_core::enums::ScriptTypeEnum;
use liteflow_core::flow::entity::InstanceInfoDto;
use liteflow_core::flow::instance_id::BaseNodeInstanceIdManageSpi;
use liteflow_core::flow::instance_id::DefaultNodeInstanceIdManageSpiImpl;
use liteflow_core::script::json_convert::{dynamic_to_json, json_to_dynamic};
use liteflow_core::script::script_bean_manager::ScriptBeanManager;
use liteflow_core::script::{RhaiScriptExecutor, ScriptExecutorComponent};
use liteflow_core::slot::Slot;
use liteflow_core::{ExecuteOption, FlowBus, FlowExecutor, NodeRef, cmp};
use serde_json::{Value, json};
use std::sync::Arc;

/// execute2RespWithRid 与 execute2RespWithEL 全参入口的真实执行。
#[tokio::test]
async fn flow_executor_rid_and_el_entries_execute_real_chains() {
    let bus = FlowBus::new();
    bus.register(
        "a",
        cmp(|ctx| async move {
            ctx.set_data("executed", json!(true));
            Ok(Value::Null)
        }),
    );
    bus.add_chain("batch_f", "THEN(a)").unwrap();

    let executor =
        FlowExecutor::new_isolated(bus.clone(), liteflow_core::LiteflowConfig::default());

    let response = executor
        .execute2_resp_with_rid(
            "batch_f",
            Value::Null,
            "RID-RID",
            vec![(
                "bean".to_string(),
                Arc::new(1_u32) as Arc<dyn std::any::Any + Send + Sync>,
            )],
        )
        .await;
    assert!(response.is_success());
    assert_eq!(response.get_request_id(), "RID-RID");
    assert_eq!(response.data("executed"), Some(json!(true)));

    let el_response = executor
        .execute_with_el_full("THEN(a)", Value::Null, Some("RID-EL".to_string()))
        .await;
    assert!(el_response.is_success());
    assert_eq!(el_response.get_request_id(), "RID-EL");
}

/// ExecuteOption 会话 ID 生成与选项装配。
#[test]
fn execute_option_conversation_and_metadata() {
    let id = liteflow_core::gen_conversation_id();
    assert!(!id.is_empty());
    // 两次生成不同
    assert_ne!(id, liteflow_core::gen_conversation_id());

    let option = ExecuteOption::of()
        .request_id("REQ-O")
        .conversation_id("CID-O");
    assert_eq!(option.get_request_id(), Some("REQ-O"));
    assert_eq!(option.get_conversation_id(), Some("CID-O"));
}

/// ScriptBeanManager 的增删查与快照。
#[test]
fn script_bean_manager_crud_and_snapshot() {
    let bean = liteflow_core::script::proxy::ScriptBeanProxy::new(
        "greeting",
        &[],
        &[],
        std::iter::empty::<liteflow_core::script::proxy::ScriptMethodProxy>(),
    );
    ScriptBeanManager::add_script_bean(bean);

    assert!(ScriptBeanManager::get_script_bean("greeting").is_some());
    assert!(ScriptBeanManager::get_script_bean("missing").is_none());
    let snapshot = ScriptBeanManager::get_script_bean_map();
    assert!(snapshot.iter().any(|(name, _)| name == "greeting"));

    ScriptBeanManager::remove_script_bean("greeting");
    assert!(ScriptBeanManager::get_script_bean("greeting").is_none());
}

/// JSON 与 Rhai Dynamic 的双向转换。
#[test]
fn json_dynamic_conversion_round_trip() {
    assert_eq!(dynamic_to_json(&json_to_dynamic(&Value::Null)), Value::Null);
    assert_eq!(dynamic_to_json(&json_to_dynamic(&json!(true))), json!(true));
    assert_eq!(dynamic_to_json(&json_to_dynamic(&json!(42))), json!(42));
    assert_eq!(dynamic_to_json(&json_to_dynamic(&json!(3.5))), json!(3.5));
    assert_eq!(
        dynamic_to_json(&json_to_dynamic(&json!("text"))),
        json!("text")
    );
    assert_eq!(
        dynamic_to_json(&json_to_dynamic(&json!([1, 2]))),
        json!([1, 2])
    );
    assert_eq!(
        dynamic_to_json(&json_to_dynamic(&json!({"k": "v"}))),
        json!({"k": "v"})
    );
}

/// 实例编号分配与从文件恢复（按节点出现次数编号）。
#[test]
fn instance_id_assign_and_restore() {
    let component: Arc<dyn liteflow_core::NodeComponent> =
        Arc::new(cmp(|_| async { Ok(Value::Null) }));
    let mut nodes = vec![
        liteflow_core::flow::element::node::Node::new(NodeRef::new("a"), component.clone()),
        liteflow_core::flow::element::node::Node::new(NodeRef::new("a"), component.clone()),
        liteflow_core::flow::element::node::Node::new(NodeRef::new("b"), component),
    ];

    let spi = DefaultNodeInstanceIdManageSpiImpl::with_base_path(
        std::env::temp_dir().join("liteflow-instance-batch-f"),
    );
    let infos = BaseNodeInstanceIdManageSpi::assign_instance_ids(&spi, &mut nodes, "chain-f");
    assert_eq!(infos.len(), 3);
    // a 出现两次，编号 a_1/a_2
    assert!(infos.iter().any(|info| info.node_id() == Some("a")));
    assert!(
        nodes
            .iter()
            .all(|node| node.get_node_instance_id().is_some())
    );

    // 恢复：用已有文件信息覆盖节点编号
    let restored_infos = vec![InstanceInfoDto::new("chain-f", "a", "a_7", 0)];
    let mut restored = vec![
        liteflow_core::flow::element::node::Node::new(
            NodeRef::new("a"),
            Arc::new(cmp(|_| async { Ok(Value::Null) })),
        ),
        liteflow_core::flow::element::node::Node::new(
            NodeRef::new("b"),
            Arc::new(cmp(|_| async { Ok(Value::Null) })),
        ),
    ];
    BaseNodeInstanceIdManageSpi::restore_instance_ids(&mut restored, "chain-f", &restored_infos);
    assert_eq!(restored[0].get_node_instance_id(), Some("a_7"));
}

/// ScriptExecutorComponent 暴露真实执行器（Rhai）。
#[test]
fn script_executor_component_exposes_executor() {
    let executor = Arc::new(RhaiScriptExecutor::default());
    let component = ScriptExecutorComponent::new(
        "exec-node",
        liteflow_core::script::ScriptKind::Common,
        executor,
    );
    assert_eq!(component.executor().script_type().get_engine_name(), "rhai");
}

/// ScriptTypeEnum 的引擎名/显示名/校验入口。
#[test]
fn script_type_enum_java_entries() {
    assert_eq!(ScriptTypeEnum::Rhai.get_engine_name(), "rhai");
    assert_eq!(ScriptTypeEnum::QlExpress.get_display_name(), "qlexpress");
    assert_eq!(
        ScriptTypeEnum::get_enum_by_display_name("qlexpress"),
        Some(ScriptTypeEnum::QlExpress)
    );
    assert!(ScriptTypeEnum::check_script_type("rhai"));
    assert!(!ScriptTypeEnum::check_script_type("unknown"));
}

/// 脚本迭代器组件用真实 Rhai 脚本返回数组。
#[tokio::test]
async fn script_iterator_component_processes_iterators() {
    let component =
        liteflow_core::script::ScriptIteratorComponent::new("iter-node", "return [1, 2, 3];")
            .expect("脚本应编译成功");
    let slot = Arc::new(Slot::new("RID-ITER".to_string(), "main", Value::Null));
    let context = liteflow_core::CmpContext {
        inner: slot.clone(),
        node: NodeRef::new("iter-node"),
        frame: liteflow_core::Frame::root(),
    };
    use liteflow_core::NodeComponent;
    let result = NodeComponent::process(&component, &context).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), json!([1, 2, 3]));
}
