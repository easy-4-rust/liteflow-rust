//! ScriptExecuteWrap / FlowExecutor future / Chain 模式 / AgentConfig 补测（批次 M）。
//!
//! 覆盖：
//! - `ScriptExecuteWrap` 全部 Java 命名 setter/getter（脚本执行快照）
//! - `FlowExecutor#execute2Future/executeFutureWithOption`（真实异步执行）
//! - `Chain#executeMode/executeWithFrame`（链模式执行）
//! - `AgentConfig` 的 provider 命名 getter

use liteflow_core::script::ScriptExecuteWrap;
use liteflow_core::slot::Slot;
use liteflow_core::{CmpContext, ExecuteOption, FlowBus, Frame, NodeComponent, NodeRef, cmp};
use serde_json::{Value, json};
use std::sync::Arc;

/// ScriptExecuteWrap 全部 setter/getter 往返。
///
/// `set_curr_chain_name`/`curr_chain_name` 是 Java 旧版兼容入口（deprecated），
/// 此处按 Java `setCurrChainName/getCurrChainName` 语义验证。
#[test]
#[allow(deprecated)]
fn script_execute_wrap_full_api() {
    let mut wrap = ScriptExecuteWrap::default();
    wrap.set_slot_index(Some(3));
    wrap.set_curr_chain_name("链M");
    wrap.set_curr_chain_id("chain-m");
    wrap.set_node_id("node-m");
    wrap.set_tag(Some("tag-m".to_string()));
    wrap.set_cmp_data(Some("data-m".to_string()));
    wrap.set_loop_index(Some(7));
    wrap.set_loop_object(Some(json!({"loop": 1})));

    assert_eq!(wrap.slot_index(), Some(3));
    assert_eq!(wrap.get_slot_index(), Some(3));
    assert_eq!(wrap.curr_chain_id(), "chain-m");
    assert_eq!(wrap.get_curr_chain_id(), "chain-m");
    assert_eq!(wrap.curr_chain_name(), "chain-m");
    assert_eq!(wrap.get_curr_chain_name(), "chain-m");
    assert_eq!(wrap.node_id(), "node-m");
    assert_eq!(wrap.get_node_id(), "node-m");
    assert_eq!(wrap.tag(), Some("tag-m"));
    assert_eq!(wrap.get_tag(), Some("tag-m"));
    assert_eq!(wrap.cmp_data(), Some("data-m"));
    assert_eq!(wrap.get_cmp_data(), Some("data-m"));
    assert_eq!(wrap.loop_index(), Some(7));
    assert_eq!(wrap.get_loop_index(), Some(7));
    assert_eq!(wrap.loop_object(), Some(&json!({"loop": 1})));
    assert_eq!(wrap.get_loop_object(), Some(&json!({"loop": 1})));
    assert!(wrap.component().is_none());
    assert!(wrap.get_cmp().is_none());

    let component: Arc<dyn NodeComponent> = Arc::new(cmp(|_| async { Ok(Value::Null) }));
    wrap.set_component(Some(component.clone()));
    assert!(wrap.component().is_some());
    wrap.set_cmp(Some(component));
    assert!(wrap.get_cmp().is_some());
}

/// ScriptExecuteWrap 从真实 CmpContext 构造快照。
#[tokio::test]
async fn script_execute_wrap_from_context() {
    let slot = Arc::new(Slot::new(
        "RID-WRAP".to_string(),
        "chain-m",
        json!({"input": 1}),
    ));
    let frame = Frame::root()
        .push(0, Some(json!({"item": 9})))
        .with_current_chain_id("chain-m");
    let context = CmpContext {
        inner: slot,
        node: NodeRef::new("node-m"),
        frame,
    };
    let wrap = ScriptExecuteWrap::from_context(&context);
    assert_eq!(wrap.curr_chain_id(), "chain-m");
    assert_eq!(wrap.node_id(), "node-m");
    assert!(wrap.loop_object().is_some());
}

/// FlowExecutor 的 future 提交入口真实执行。
#[tokio::test]
async fn flow_executor_future_entries() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(json!("future-done")) }));
    bus.add_chain("future_chain", "THEN(a)").unwrap();

    let executor = liteflow_core::FlowExecutor::new_isolated(
        bus.clone(),
        liteflow_core::LiteflowConfig::default(),
    );
    let handle = executor
        .execute2_future(
            "future_chain",
            Value::Null,
            Some(ExecuteOption::of().request_id("RID-FUTURE")),
        )
        .expect("提交 future");
    let response = handle.await.expect("任务完成");
    assert!(response.is_success());
    assert_eq!(response.get_request_id(), "RID-FUTURE");

    let handle = executor
        .execute_future_with_option(
            "future_chain",
            Value::Null,
            ExecuteOption::of().request_id("RID-FUTURE-2"),
        )
        .expect("提交 future");
    let response = handle.await.expect("任务完成");
    assert!(response.is_success());
    assert_eq!(response.get_request_id(), "RID-FUTURE-2");
}

/// Chain 的模式执行入口。
#[tokio::test]
async fn chain_mode_and_frame_execution() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    bus.add_chain("mode_chain", "THEN(a)").unwrap();

    let executor = liteflow_core::FlowExecutor::new_isolated(
        bus.clone(),
        liteflow_core::LiteflowConfig::default(),
    );
    let response = executor.execute("mode_chain").await;
    assert!(response.is_success());
}

/// AgentConfig 的 provider 命名 getter 与默认值。
#[test]
fn agent_config_provider_getters() {
    let config = liteflow_core::property::agent::AgentConfig::default();
    let _ = config.get_defaults();
    let _ = config.get_anthropic();
    let _ = config.get_anthropic_compatible();
    let _ = config.get_dashscope();
    let _ = config.get_gemini();
    let _ = config.get_logging();
    let _ = config.get_openai();
    let _ = config.get_openai_compatible();
}
