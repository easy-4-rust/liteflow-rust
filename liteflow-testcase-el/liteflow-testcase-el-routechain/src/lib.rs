//! RouteChain 决策表集成场景。

use liteflow_core::{FlowBus, cmp};
use serde_json::Value;

/// 构建并注册带 namespace 的 route chain。
pub async fn run_case() -> bool {
    let bus = FlowBus::new();
    bus.register("route_check", cmp(|_| async { Ok(Value::Bool(true)) }));
    bus.register("route_body", cmp(|_| async { Ok(Value::Null) }));
    bus.add_route_chain("route_chain", "testcase", "route_check", "THEN(route_body)")
        .is_ok()
}
