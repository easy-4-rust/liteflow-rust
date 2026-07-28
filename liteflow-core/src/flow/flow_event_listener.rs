//! LiteFlow 执行期间的事件监听接口。

use crate::flow::flow_event::FlowEvent;

use super::closure_listener::ClosureListener;

/// 接收一次 LiteFlow 执行期间发布的业务事件。
///
/// 监听器随 ExecuteOption 传入，只在本次执行的 Slot 内生效。
/// 对应 Java: `com.yomahub.liteflow.flow.FlowEventListener`。
pub trait FlowEventListener: Send + Sync {
    /// 处理一个流程事件。
    ///
    /// 参数 `event` 是组件通过当前 Slot 发布的事件。
    /// 对应 Java: `FlowEventListener#onEvent`。
    fn on_event(&self, event: &FlowEvent);
}

/// 将线程安全闭包适配为事件监听器。
///
/// 参数 `f` 在收到每个 FlowEvent 时被调用；返回值可直接写入 ExecuteOption。
pub fn listener<F: Fn(&FlowEvent) + Send + Sync + 'static>(f: F) -> impl FlowEventListener {
    ClosureListener(f)
}
