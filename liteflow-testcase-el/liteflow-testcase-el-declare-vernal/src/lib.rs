//! 单声明式组件的 Vernal EL 场景。

use liteflow_core::parse_el;

/// 校验 `cmpId.methodName` 声明式方法引用。
pub async fn run_case() -> bool {
    parse_el("THEN(order_service.create)").is_ok()
}
