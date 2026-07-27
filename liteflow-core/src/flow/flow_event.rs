//! LiteFlow 执行事件及其伴随 Builder。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// LiteFlow 执行期间向调用方推送的通用事件。
///
/// 事件不绑定具体组件类型。普通组件、Agent 组件和插件都可以通过
/// `FlowEventPublisher` 发布，调用方通过 `ExecuteOption` 注册监听器。
///
/// 对应 Java: `com.yomahub.liteflow.flow.FlowEvent`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowEvent {
    /// 事件类型；JSON 字段保持 Java 的 `type`。
    #[serde(rename = "type")]
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
    /// 创建空事件构建器。对应 Java: `FlowEvent#builder`。
    #[must_use]
    pub fn builder() -> FlowEventBuilder {
        FlowEventBuilder {
            event: FlowEvent {
                event_type: String::new(),
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

    /// 返回事件类型。对应 Java: `FlowEvent#getType`。
    #[must_use]
    pub fn get_type(&self) -> &str {
        &self.event_type
    }

    /// 返回 Chain ID。对应 Java: `FlowEvent#getChainId`。
    #[must_use]
    pub fn get_chain_id(&self) -> Option<&str> {
        self.chain_id.as_deref()
    }

    /// 返回节点 ID。对应 Java: `FlowEvent#getNodeId`。
    #[must_use]
    pub fn get_node_id(&self) -> Option<&str> {
        self.node_id.as_deref()
    }

    /// 返回请求 ID。对应 Java: `FlowEvent#getRequestId`。
    #[must_use]
    pub fn get_request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    /// 返回会话 ID。对应 Java: `FlowEvent#getConversationId`。
    #[must_use]
    pub fn get_conversation_id(&self) -> Option<&str> {
        self.conversation_id.as_deref()
    }

    /// 返回文本载荷。对应 Java: `FlowEvent#getText`。
    #[must_use]
    pub fn get_text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    /// 判断是否为流式输出最后一条事件。对应 Java: `FlowEvent#isLast`。
    #[must_use]
    pub fn is_last(&self) -> bool {
        self.last
    }

    /// 返回结构化数据载荷。对应 Java: `FlowEvent#getData`。
    #[must_use]
    pub fn get_data(&self) -> Option<&Value> {
        self.data.as_ref()
    }

    /// 返回毫秒时间戳。对应 Java: `FlowEvent#getTimestamp`。
    #[must_use]
    pub fn get_timestamp(&self) -> u64 {
        self.timestamp
    }
}

/// `FlowEvent` 的伴随链式构建器。
///
/// 这是 Java `FlowEvent.Builder` 静态内部类，按项目规范与主对象保存在同一文件。
pub struct FlowEventBuilder {
    event: FlowEvent,
}

impl FlowEventBuilder {
    /// 设置事件类型。
    ///
    /// `type` 是 Rust 关键字，因此使用原始标识符 `r#type`；调用语义仍是一一对应
    /// 的 `.type(value)`。对应 Java: `FlowEvent.Builder#type`。
    #[must_use]
    pub fn r#type(mut self, event_type: impl Into<String>) -> Self {
        self.event.event_type = event_type.into();
        self
    }

    /// 设置 Chain ID。对应 Java: `FlowEvent.Builder#chainId`。
    #[must_use]
    pub fn chain_id(mut self, chain_id: impl Into<String>) -> Self {
        self.event.chain_id = Some(chain_id.into());
        self
    }

    /// 设置节点 ID。对应 Java: `FlowEvent.Builder#nodeId`。
    #[must_use]
    pub fn node_id(mut self, node_id: impl Into<String>) -> Self {
        self.event.node_id = Some(node_id.into());
        self
    }

    /// 设置请求 ID。对应 Java: `FlowEvent.Builder#requestId`。
    #[must_use]
    pub fn request_id(mut self, request_id: impl Into<String>) -> Self {
        self.event.request_id = Some(request_id.into());
        self
    }

    /// 设置会话 ID。对应 Java: `FlowEvent.Builder#conversationId`。
    #[must_use]
    pub fn conversation_id(mut self, conversation_id: impl Into<String>) -> Self {
        self.event.conversation_id = Some(conversation_id.into());
        self
    }

    /// 设置文本载荷。对应 Java: `FlowEvent.Builder#text`。
    #[must_use]
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.event.text = Some(text.into());
        self
    }

    /// 设置是否为最后一条事件。对应 Java: `FlowEvent.Builder#last`。
    #[must_use]
    pub fn last(mut self, last: bool) -> Self {
        self.event.last = last;
        self
    }

    /// 设置结构化数据载荷。
    ///
    /// Java `Object` 映射为 `serde_json::Value`。对应 Java:
    /// `FlowEvent.Builder#data`。
    #[must_use]
    pub fn data(mut self, data: Value) -> Self {
        self.event.data = Some(data);
        self
    }

    /// 设置毫秒时间戳。对应 Java: `FlowEvent.Builder#timestamp`。
    #[must_use]
    pub fn timestamp(mut self, timestamp: u64) -> Self {
        self.event.timestamp = timestamp;
        self
    }

    /// 构建不可变事件。
    ///
    /// 时间戳为 0 时使用当前 Unix 毫秒数，与 Java 构造器的
    /// `System.currentTimeMillis()` 规则一致。对应 Java:
    /// `FlowEvent.Builder#build`。
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
