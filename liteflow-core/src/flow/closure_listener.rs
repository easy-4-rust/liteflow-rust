//! 闭包形式的执行事件监听器。

use super::{FlowEvent, FlowEventListener};

/// 把闭包适配为 `FlowEventListener`。
///
/// 对应 Java: `FlowEventListener` 的单抽象方法匿名实现。
pub(super) struct ClosureListener<F>(pub(super) F);

impl<F: Fn(&FlowEvent) + Send + Sync> FlowEventListener for ClosureListener<F> {
    fn on_event(&self, event: &FlowEvent) {
        (self.0)(event);
    }
}
