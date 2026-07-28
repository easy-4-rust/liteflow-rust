//! 当前执行 Slot 上的事件监听器挂载与发布工具。

use crate::flow::flow_event::FlowEvent;
use crate::flow::flow_event_listener::FlowEventListener;
use crate::slot::Ctx;
use std::sync::Arc;

/// Slot attachment 中保存事件监听器的内部键。
pub const LISTENER_KEY: &str = "_flow_event_listener";

/// 管理当前 Slot 的 FlowEventListener 并发布 FlowEvent。
///
/// 监听器保存在 Slot attachment 中，不进入全局状态；执行完成后移除。
/// 对应 Java: `com.yomahub.liteflow.flow.FlowEventPublisher`。
pub struct FlowEventPublisher;

impl FlowEventPublisher {
    /// 给当前执行数据槽挂载事件监听器。
    ///
    /// 参数 `ctx` 持有目标 Slot；`listener` 是本次执行独占的监听器。
    /// 对应 Java: `FlowEventPublisher#setListener`。
    pub fn set_listener(ctx: &Ctx, listener: Arc<dyn FlowEventListener>) {
        ctx.inner.set_attachment(LISTENER_KEY, listener);
    }

    /// 返回当前数据槽是否已经挂载事件监听器。
    ///
    /// 参数 `ctx` 持有待查询 Slot。对应 Java:
    /// `FlowEventPublisher#hasListener`。
    #[must_use]
    pub fn has_listener(ctx: &Ctx) -> bool {
        ctx.inner.has_attachment(LISTENER_KEY)
    }

    /// 移除当前数据槽上的事件监听器。
    ///
    /// 参数 `ctx` 持有目标 Slot；监听器不存在时静默完成。
    /// 对应 Java: `FlowEventPublisher#removeListener`。
    pub fn remove_listener(ctx: &Ctx) {
        ctx.inner.remove_attachment(LISTENER_KEY);
    }

    /// 向当前数据槽上的监听器发布事件。
    ///
    /// 参数 `event` 是待发布事件；没有监听器时静默忽略。
    /// 对应 Java: `FlowEventPublisher#publish`。
    pub fn publish(ctx: &Ctx, event: &FlowEvent) {
        Self::publish_ctx(&ctx.inner, event);
    }

    /// 通过共享 Slot 句柄直接发布事件。
    ///
    /// 这是供 CmpContext 等 Rust 内部对象使用的等价入口；监听器不存在时不执行。
    pub fn publish_ctx(slot: &Arc<crate::slot::Slot>, event: &FlowEvent) {
        if let Some(l) = slot.get_attachment::<Arc<dyn FlowEventListener>>(LISTENER_KEY) {
            l.on_event(event);
        }
    }
}

impl Ctx {
    /// 从组件执行上下文发布事件。
    ///
    /// 参数 `event` 转交当前 Slot 的监听器，对应组件内调用
    /// `FlowEventPublisher.publish(getSlot(), event)`。
    pub fn publish_event(&self, event: &FlowEvent) {
        FlowEventPublisher::publish(self, event);
    }
}
