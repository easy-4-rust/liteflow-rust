//! 对应 flow.FlowEventListener（2.15+）：执行事件监听接口。

use crate::flow::flow_event::FlowEvent;

use super::closure_listener::ClosureListener;

/// 事件监听器，随 ExecuteOption 传入，本次执行内生效
pub trait FlowEventListener: Send + Sync {
    fn on_event(&self, event: &FlowEvent);
}

/// 闭包便捷构造
pub fn listener<F: Fn(&FlowEvent) + Send + Sync + 'static>(f: F) -> impl FlowEventListener {
    ClosureListener(f)
}
