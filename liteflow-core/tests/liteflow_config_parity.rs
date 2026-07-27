//! LiteflowConfig 与 FlowExecutor 配置消费链路的集成测试。

use liteflow_core::{DataBus, FlowBus, FlowExecutor, LiteflowConfig};

/// 验证显式配置创建执行器时，Slot 容量真正进入 DataBus。
///
/// 对应 Java: `FlowExecutor(LiteflowConfig)` 与 `DataBus#init`。
#[test]
fn flow_executor_uses_configured_slot_size_to_initialize_data_bus() {
    let mut config = LiteflowConfig::default();
    config.set_slot_size(7);

    let _executor = FlowExecutor::new_with_config(FlowBus::new(), config);

    assert_eq!(DataBus::total_size(), 7);
}
