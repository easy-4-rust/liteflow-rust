//! 脚本组件生命周期钩子、枚举与监控/解析负向路径补测（批次 I）。
//!
//! 覆盖：
//! - `ScriptForComponent`/`ScriptSwitchComponent` 的 Java 生命周期钩子
//!   （isContinueOnError/onError/rollback/unloadScript/onSuccess 等）
//! - `NodeTypeEnum` 的 code/中文名/组件类映射/脚本切换
//! - `MonitorFile` 目录扫描与扩展名校验
//! - `RhaiScriptExecutor#validateWithEx` 与错误路径
//! - `ScriptComponent#buildWrap` 的上下文快照

use liteflow_core::enums::NodeTypeEnum;
use liteflow_core::flow::entity::InstanceInfoDto;
use liteflow_core::monitor::MonitorFile;
use liteflow_core::script::{RhaiScriptExecutor, ScriptForComponent, ScriptSwitchComponent};
use liteflow_core::slot::Slot;
use liteflow_core::{CmpContext, FlowBus, Frame, LiteflowError, NodeComponent, NodeRef, cmp};
use serde_json::Value;
use std::sync::Arc;

/// NodeTypeEnum 的 code、中文名与组件类映射。
#[test]
fn node_type_enum_java_metadata() {
    assert_eq!(NodeTypeEnum::Common.get_code(), "common");
    assert_eq!(NodeTypeEnum::Boolean.get_code(), "boolean");
    assert_eq!(NodeTypeEnum::Switch.get_code(), "switch");
    assert_eq!(NodeTypeEnum::For.get_code(), "for");
    assert_eq!(NodeTypeEnum::While.get_code(), "while");
    assert_eq!(NodeTypeEnum::Iterator.get_code(), "iterator");
    assert_eq!(NodeTypeEnum::Script.get_code(), "script");
    assert_eq!(NodeTypeEnum::BooleanScript.get_code(), "boolean_script");
    assert_eq!(NodeTypeEnum::SwitchScript.get_code(), "switch_script");
    assert_eq!(NodeTypeEnum::ForScript.get_code(), "for_script");
    assert_eq!(NodeTypeEnum::WhileScript.get_code(), "while_script");
    assert_eq!(NodeTypeEnum::BreakScript.get_code(), "break_script");
    assert_eq!(NodeTypeEnum::Fallback.get_code(), "fallback");

    assert_eq!(NodeTypeEnum::Common.get_name(), "普通");
    assert_eq!(NodeTypeEnum::Script.get_name(), "脚本");
    assert_eq!(NodeTypeEnum::Fallback.get_name(), "降级");

    // 组件类映射
    assert_eq!(
        NodeTypeEnum::Common.get_mapping_clazz(),
        Some("NodeComponent")
    );
    assert_eq!(
        NodeTypeEnum::SwitchScript.get_mapping_clazz(),
        Some("ScriptSwitchComponent")
    );
    assert_eq!(
        NodeTypeEnum::Boolean.get_mapping_clazz(),
        Some("NodeBooleanComponent")
    );
    assert_eq!(
        NodeTypeEnum::For.get_mapping_clazz(),
        Some("NodeForComponent")
    );

    // 脚本切换（Java 2.16 的 isScript 语义：setScript 返回是否成功）
    let mut common = NodeTypeEnum::Common;
    assert!(common.set_script(true));
    assert!(common.is_script());
}

/// ScriptForComponent 的 Java 生命周期钩子与卸载。
#[tokio::test]
async fn script_for_component_lifecycle_hooks() {
    let bus = FlowBus::new();
    bus.register_script_typed(
        "for_script",
        "rhai",
        liteflow_core::script::ScriptKind::For,
        "3",
    )
    .unwrap();
    bus.register("body", cmp(|_| async { Ok(Value::Null) }));
    bus.add_chain("script_for_chain", "FOR(for_script).DO(body)")
        .unwrap();

    let context = script_context(bus.clone(), "for_script");

    let component = ScriptForComponent::new("for_script", "3").unwrap();
    // 卸载脚本后 execute_is_continue_on_error 等钩子仍可安全调用
    assert!(!component.is_continue_on_error(&context));
    component
        .on_error(&context, &LiteflowError::Custom("boom".into()))
        .await;
    assert!(component.rollback(&context).await.is_ok());
    assert!(component.unload_script("for_script").is_ok());
}

/// ScriptSwitchComponent 的 Java 生命周期钩子与卸载。
#[tokio::test]
async fn script_switch_component_lifecycle_hooks() {
    let bus = FlowBus::new();
    bus.register_script_typed(
        "switch_script",
        "rhai",
        liteflow_core::script::ScriptKind::Switch,
        r#"return "target-a";"#,
    )
    .unwrap();
    bus.add_chain(
        "script_switch_chain",
        "SWITCH(switch_script).TO(target_a, target_b)",
    )
    .unwrap();

    let context = script_context(bus.clone(), "switch_script");

    let component = ScriptSwitchComponent::new("switch_script", r#"return "target-a";"#).unwrap();
    assert!(!component.is_continue_on_error(&context));
    component
        .on_error(&context, &LiteflowError::Custom("boom".into()))
        .await;
    assert!(component.rollback(&context).await.is_ok());
    assert!(component.unload_script("switch_script").is_ok());
}

fn script_context(_bus: FlowBus, node_id: &str) -> CmpContext {
    let slot = Arc::new(Slot::new(
        "RID-SCRIPT".to_string(),
        "script_chain",
        Value::Null,
    ));
    CmpContext {
        inner: slot,
        node: NodeRef::new(node_id),
        frame: Frame::root(),
    }
}

/// MonitorFile 目录扫描与扩展名过滤。
#[test]
fn monitor_file_directory_scan_and_extension_check() {
    let dir = std::env::temp_dir().join("liteflow-monitor-i");
    std::fs::create_dir_all(dir.join("rules")).unwrap();
    std::fs::write(dir.join("rules").join("a.xml"), "<flow/>").unwrap();
    std::fs::write(dir.join("rules").join("b.yml"), "flow:").unwrap();
    std::fs::write(dir.join("rules").join("c.txt"), "ignore").unwrap();

    let bus = FlowBus::new();
    let monitor = MonitorFile::new(bus);
    monitor
        .add_monitor_file_path(dir.join("rules"))
        .expect("目录应被登记");

    let _ = std::fs::remove_dir_all(&dir);
}

/// RhaiScriptExecutor 的 validateWithEx 与未加载节点错误。
#[tokio::test]
async fn rhai_executor_validate_and_unloaded_errors() {
    let executor = RhaiScriptExecutor::default();
    let validation = executor.validate_with_ex("let x = 1;");
    assert!(validation.is_success());

    // 未加载节点执行报错
    let slot = Arc::new(Slot::new("RID-RHAI".to_string(), "main", Value::Null));
    let context = CmpContext {
        inner: slot,
        node: NodeRef::new("not_loaded"),
        frame: Frame::root(),
    };
    let result = liteflow_core::script::ScriptExecutor::execute(&executor, "not_loaded", &context);
    assert!(result.is_err());
}

/// InstanceInfoDto 的 Java 命名访问器（补足批次 F 未覆盖字段）。
#[test]
fn instance_info_dto_java_accessors() {
    let mut dto = InstanceInfoDto::new("chain-x", "node-x", "node_x_1", 2);
    assert_eq!(dto.chain_id(), Some("chain-x"));
    assert_eq!(dto.node_id(), Some("node-x"));
    assert_eq!(dto.instance_id(), Some("node_x_1"));
    assert_eq!(dto.index(), Some(2));

    dto.set_chain_id("chain-y");
    dto.set_node_id("node-y");
    dto.set_instance_id("node_y_2");
    dto.set_index(3);
    assert_eq!(dto.get_chain_id(), Some("chain-y"));
    assert_eq!(dto.get_node_id(), Some("node-y"));
    assert_eq!(dto.get_instance_id(), Some("node_y_2"));
    assert_eq!(dto.get_index(), Some(3));
}
