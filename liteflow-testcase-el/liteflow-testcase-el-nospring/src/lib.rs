//! 无容器 LiteFlow baseline 集成场景。

use liteflow_core::{FlowBus, cmp};
use serde_json::{Value, json};

/// 执行无 Spring/Vernal 容器的基础链路。
pub async fn run_case() -> bool {
    let bus = FlowBus::new();
    bus.register(
        "nospring_node",
        cmp(|ctx| async move {
            ctx.set_data("nospring", json!(true));
            Ok(Value::Null)
        }),
    );
    bus.add_chain("nospring_chain", "THEN(nospring_node)")
        .is_ok()
        && bus.execute("nospring_chain").await.is_success()
}
