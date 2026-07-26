//! 多声明式方法的 Vernal EL 场景。

use liteflow_core::parse_el;

/// 校验同一声明式组件的多个方法引用可组成链路。
pub async fn run_case() -> bool {
    parse_el("THEN(order_service.validate, order_service.create)").is_ok()
}
