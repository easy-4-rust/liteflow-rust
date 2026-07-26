//! 对应 Java FlowEvent 的 Builder。

use serde_json::Value;

use super::FlowEvent;

/// `FlowEvent` 链式构建器。
pub struct FlowEventBuilder {
    pub(crate) event: FlowEvent,
}

impl FlowEventBuilder {
    /// 设置链 id。
    #[must_use]
    pub fn chain_id(mut self, chain_id: impl Into<String>) -> Self {
        self.event.chain_id = Some(chain_id.into());
        self
    }
    /// 设置节点 id。
    #[must_use]
    pub fn node_id(mut self, node_id: impl Into<String>) -> Self {
        self.event.node_id = Some(node_id.into());
        self
    }
    /// 设置请求 id。
    #[must_use]
    pub fn request_id(mut self, request_id: impl Into<String>) -> Self {
        self.event.request_id = Some(request_id.into());
        self
    }
    /// 设置会话 id。
    #[must_use]
    pub fn conversation_id(mut self, conversation_id: impl Into<String>) -> Self {
        self.event.conversation_id = Some(conversation_id.into());
        self
    }
    /// 设置文本载荷。
    #[must_use]
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.event.text = Some(text.into());
        self
    }
    /// 设置是否为最后一条事件。
    #[must_use]
    pub fn last(mut self, last: bool) -> Self {
        self.event.last = last;
        self
    }
    /// 设置结构化数据载荷。
    #[must_use]
    pub fn data(mut self, data: Value) -> Self {
        self.event.data = Some(data);
        self
    }
    /// 设置毫秒时间戳。
    #[must_use]
    pub fn timestamp(mut self, timestamp: u64) -> Self {
        self.event.timestamp = timestamp;
        self
    }
    /// 完成事件构建，时间戳缺省时使用当前系统时间。
    #[must_use]
    pub fn build(mut self) -> FlowEvent {
        if self.event.timestamp == 0 {
            self.event.timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_millis() as u64)
                .unwrap_or(0);
        }
        self.event
    }
}
