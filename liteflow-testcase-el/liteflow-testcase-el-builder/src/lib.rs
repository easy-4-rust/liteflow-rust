//! EL Builder API 集成场景。

use liteflow_el_builder::{ELBus, ELWrapper};

/// 构建与 Java ELBus 一致的 THEN 表达式。
pub async fn run_case() -> bool {
    ELBus::then(["builder_a", "builder_b"])
        .and_then(|wrapper| wrapper.to_expression())
        .is_ok_and(|expression| expression == "THEN(builder_a,builder_b)")
}
