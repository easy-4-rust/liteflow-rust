//! 对应 flow.FlowEvent（2.15+）：执行事件。
//! 主要服务于 ReAct Agent / 业务侧观测场景：组件或 Agent 通过
//! FlowEventPublisher 把事件推给本次执行注册的 FlowEventListener。

use serde_json::Value;

#[derive(Debug, Clone)]
pub struct FlowEvent {
    pub event_type: String,
    pub chain_id: Option<String>,
    pub node_id: Option<String>,
    pub request_id: Option<String>,
    pub conversation_id: Option<String>,
    pub text: Option<String>,
    /// 是否为最后一条事件（流式输出结束标记）
    pub last: bool,
    pub data: Option<Value>,
    /// 毫秒时间戳（缺省取当前时间）
    pub timestamp: u64,
}

impl FlowEvent {
    pub fn builder(event_type: impl Into<String>) -> FlowEventBuilder {
        FlowEventBuilder {
            event: FlowEvent {
                event_type: event_type.into(),
                chain_id: None,
                node_id: None,
                request_id: None,
                conversation_id: None,
                text: None,
                last: false,
                data: None,
                timestamp: 0,
            },
        }
    }
}

pub struct FlowEventBuilder {
    event: FlowEvent,
}

impl FlowEventBuilder {
    pub fn chain_id(mut self, chain_id: impl Into<String>) -> Self {
        self.event.chain_id = Some(chain_id.into());
        self
    }
    pub fn node_id(mut self, node_id: impl Into<String>) -> Self {
        self.event.node_id = Some(node_id.into());
        self
    }
    pub fn request_id(mut self, request_id: impl Into<String>) -> Self {
        self.event.request_id = Some(request_id.into());
        self
    }
    pub fn conversation_id(mut self, conversation_id: impl Into<String>) -> Self {
        self.event.conversation_id = Some(conversation_id.into());
        self
    }
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.event.text = Some(text.into());
        self
    }
    pub fn last(mut self, last: bool) -> Self {
        self.event.last = last;
        self
    }
    pub fn data(mut self, data: Value) -> Self {
        self.event.data = Some(data);
        self
    }
    pub fn timestamp(mut self, timestamp: u64) -> Self {
        self.event.timestamp = timestamp;
        self
    }
    pub fn build(mut self) -> FlowEvent {
        if self.event.timestamp == 0 {
            self.event.timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
        }
        self.event
    }
}
