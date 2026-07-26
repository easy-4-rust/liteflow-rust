//! 对应 core.ExecuteOption（2.16 新增）：一次执行的选项。
//! requestId / conversationId / contextBeans / eventListener。

use crate::flow::flow_event_listener::FlowEventListener;
use crate::util::ConversationIdGenerator;
use std::any::Any;
use std::sync::Arc;

/// 生成默认会话 ID。
///
/// 保留既有函数入口并委托独立的 `ConversationIdGenerator` 对象。
pub fn gen_conversation_id() -> String {
    ConversationIdGenerator::generate()
}

#[derive(Default, Clone)]
pub struct ExecuteOption {
    /// 指定本次执行的 requestId（None 时框架自动生成）
    pub request_id: Option<String>,
    /// 业务会话标识（ReAct Agent 连续对话场景）
    pub conversation_id: Option<String>,
    /// 声明需要 conversationId 但由框架生成
    pub auto_conversation_id: bool,
    /// contextBean 列表
    pub context_beans: Vec<(String, Arc<dyn Any + Send + Sync>)>,
    /// 本次执行的事件监听器
    pub event_listener: Option<Arc<dyn FlowEventListener>>,
}

impl ExecuteOption {
    pub fn of() -> Self {
        Self::default()
    }
    pub fn request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }
    /// 显式设置 conversationId（取消 auto_conversation_id 语义）
    pub fn conversation_id(mut self, conversation_id: impl Into<String>) -> Self {
        self.conversation_id = Some(conversation_id.into());
        self.auto_conversation_id = false;
        self
    }
    pub fn auto_conversation_id(mut self) -> Self {
        self.auto_conversation_id = true;
        self.conversation_id = None;
        self
    }
    pub fn context_bean(
        mut self,
        name: impl Into<String>,
        bean: Arc<dyn Any + Send + Sync>,
    ) -> Self {
        self.context_beans.push((name.into(), bean));
        self
    }
    pub fn event_listener(mut self, listener: Arc<dyn FlowEventListener>) -> Self {
        self.event_listener = Some(listener);
        self
    }
    /// 解析最终 conversationId（对应 FlowExecutor 内 resolve 逻辑）
    pub(crate) fn resolve_conversation_id(&self) -> Option<String> {
        if let Some(cid) = &self.conversation_id {
            Some(cid.clone())
        } else if self.auto_conversation_id {
            Some(gen_conversation_id())
        } else {
            None
        }
    }
}
