//! 对应 flow.FlowEventPublisher（2.15+）：
//! listener 存于 Slot attachment（LISTENER_KEY），publish 时取出并回调。

use crate::flow::flow_event::FlowEvent;
use crate::flow::flow_event_listener::FlowEventListener;
use crate::slot::Ctx;
use std::sync::Arc;

pub const LISTENER_KEY: &str = "_flow_event_listener";

pub struct FlowEventPublisher;

impl FlowEventPublisher {
    /// setListener(slot, listener)
    pub fn set_listener(ctx: &Ctx, listener: Arc<dyn FlowEventListener>) {
        ctx.inner.set_attachment(LISTENER_KEY, listener);
    }
    /// hasListener(slot)
    pub fn has_listener(ctx: &Ctx) -> bool {
        ctx.inner.has_attachment(LISTENER_KEY)
    }
    /// removeListener(slot)
    pub fn remove_listener(ctx: &Ctx) {
        ctx.inner.remove_attachment(LISTENER_KEY);
    }
    /// publish(slot, event)：无 listener 时静默忽略（对齐 Java）
    pub fn publish(ctx: &Ctx, event: &FlowEvent) {
        Self::publish_ctx(&ctx.inner, event);
    }

    /// 以 Slot 直接发布（CmpContext 等持有 Arc<Slot> 的场景）
    pub fn publish_ctx(slot: &Arc<crate::slot::Slot>, event: &FlowEvent) {
        if let Some(l) = slot.get_attachment::<Arc<dyn FlowEventListener>>(LISTENER_KEY) {
            l.on_event(event);
        }
    }
}

impl Ctx {
    /// publish 便捷方法（对应组件内 FlowEventPublisher.publish(getSlot(), event)）
    pub fn publish_event(&self, event: &FlowEvent) {
        FlowEventPublisher::publish(self, event);
    }
}
