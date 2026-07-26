//! 对应 flow.FlowEvent（2.15+）：执行事件。
//! 主要服务于 ReAct Agent / 业务侧观测场景：组件或 Agent 通过
//! FlowEventPublisher 把事件推给本次执行注册的 FlowEventListener。

use super::FlowEventBuilder;
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
