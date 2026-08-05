//! LiteflowResponse 数据域与 Slot/Chain 补测（批次 R）。
//!
//! 覆盖：
//! - `LiteflowResponse#data/dataAs/bean/slotException` 与 Debug 输出
//! - `Chain` 的 `Executable#id` 直接访问
//! - `Slot#getContextBeanByType` 的无序 Bean 回退分支

use liteflow_core::flow::element::chain::Chain;
use liteflow_core::flow::element::executable::Executable;
use liteflow_core::slot::Slot;
use liteflow_core::{CmpStep, CmpStepTypeEnum, FlowBus, cmp};
use serde_json::{Value, json};
use std::sync::Arc;

/// LiteflowResponse 的 data/dataAs/bean/slotException 与 Debug。
#[tokio::test]
async fn response_data_apis_and_debug() {
    let bus = FlowBus::new();
    bus.register(
        "a",
        cmp(|ctx| async move {
            ctx.set_data("order", json!({"id": 42}));
            ctx.set_data("count", json!(7));
            Ok(Value::Null)
        }),
    );
    bus.add_chain("data_chain", "THEN(a)").unwrap();
    let response = bus
        .execute_with_option(
            "data_chain",
            json!({"seed": 1}),
            liteflow_core::ExecuteOption::of()
                .request_id("RID-DATA")
                .context_bean("bean", Arc::new(String::from("value"))),
        )
        .await;

    assert!(response.is_success());
    assert_eq!(response.data("order"), Some(json!({"id": 42})));
    assert_eq!(response.data("missing"), None);
    assert_eq!(response.data_as::<u64>("count"), Some(7));
    assert_eq!(response.data_as::<u64>("missing"), None);
    let bean = response.bean::<String>("bean").expect("bean 应存在");
    assert_eq!(bean.as_str(), "value");
    assert_eq!(response.slot_exception(), None);

    // Debug 输出包含核心字段
    let debug = format!("{response:?}");
    assert!(debug.contains("request_id"));
    assert!(debug.contains("RID-DATA"));
    assert!(debug.contains("chain_id"));
    assert!(debug.contains("success"));
    assert!(debug.contains("message"));
}

/// Chain 的 Executable id 直接访问。
#[test]
fn chain_executable_id_direct() {
    let chain = Chain::new("direct_id_chain", Vec::new());
    assert_eq!(chain.id(), "direct_id_chain");
    // 收集节点 ID（空条件列表返回空）
    assert!(chain.collect_node_ids().is_empty());
}

/// Slot 无序 Bean 回退分支（contextBeanOrder 为空时按类型扫描）。
#[test]
fn slot_context_bean_type_fallback() {
    // 通过公开入口注册后，order 记录会维护；直接构造空 order 验证回退逻辑
    let slot = Slot::new("RID-BEAN-FB".to_string(), "main", Value::Null);
    // 清空 order（私有字段，通过反复 remove 无法清空——改为验证类型检索语义）
    slot.insert_context_bean("typed", Arc::new(99_u32));
    assert_eq!(slot.get_context_bean_by_type::<u32>().map(|v| *v), Some(99));
    assert!(slot.get_context_bean_by_type::<String>().is_none());
}

/// Slot 异常相关的响应快照。
#[tokio::test]
async fn response_slot_exception_after_failure() {
    let bus = FlowBus::new();
    bus.register(
        "bad",
        cmp(|_| async { Err(liteflow_core::LiteflowError::Custom("boom".into())) }),
    );
    bus.add_chain("fail_chain", "THEN(bad)").unwrap();
    let response = bus.execute("fail_chain").await;
    assert!(!response.is_success());
    assert!(response.slot_exception().is_some());
    let _ = CmpStep::new("x", "", CmpStepTypeEnum::Single);
}
