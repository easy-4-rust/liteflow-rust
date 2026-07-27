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

/// 单次链路执行选项。
///
/// 保存请求标识、会话标识、上下文 Bean 与事件监听器。字段使用 Rust 所有权
/// 表达 Java Builder 的可选值，并保留与 Java getter 对应的 snake_case 方法。
/// 对应 Java: `com.yomahub.liteflow.core.ExecuteOption`。
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
    /// 创建空执行选项。对应 Java: `ExecuteOption#of`。
    #[must_use]
    pub fn of() -> Self {
        Self::default()
    }

    /// 设置请求 ID。
    ///
    /// 参数 `request_id` 对应 Java `requestId`。
    #[must_use]
    pub fn request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// 显式设置会话 ID，并取消自动生成语义。
    ///
    /// 参数 `conversation_id` 对应 Java `conversationId`。
    #[must_use]
    pub fn conversation_id(mut self, conversation_id: impl Into<String>) -> Self {
        self.conversation_id = Some(conversation_id.into());
        self.auto_conversation_id = false;
        self
    }

    /// 请求框架自动生成会话 ID。对应 Java: `ExecuteOption#autoConversationId`。
    #[must_use]
    pub fn auto_conversation_id(mut self) -> Self {
        self.auto_conversation_id = true;
        self.conversation_id = None;
        self
    }

    /// 添加一个具名上下文 Bean。
    ///
    /// Java 可按 Class 或实例传入；Rust 使用名称与线程安全类型擦除对象表达同一
    /// 运行时能力。参数 `name`、`bean` 与 Java 上下文 Bean 语义一致。
    #[must_use]
    pub fn context_bean(
        mut self,
        name: impl Into<String>,
        bean: Arc<dyn Any + Send + Sync>,
    ) -> Self {
        self.context_beans.push((name.into(), bean));
        self
    }

    /// 设置本次执行的事件监听器。对应 Java: `ExecuteOption#eventListener`。
    #[must_use]
    pub fn event_listener(mut self, listener: Arc<dyn FlowEventListener>) -> Self {
        self.event_listener = Some(listener);
        self
    }

    /// 返回请求 ID。对应 Java: `ExecuteOption#getRequestId`。
    #[must_use]
    pub fn get_request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    /// 返回显式会话 ID。对应 Java: `ExecuteOption#getConversationId`。
    #[must_use]
    pub fn get_conversation_id(&self) -> Option<&str> {
        self.conversation_id.as_deref()
    }

    /// 返回是否自动生成会话 ID。对应 Java: `ExecuteOption#isAutoConversationId`。
    #[must_use]
    pub fn is_auto_conversation_id(&self) -> bool {
        self.auto_conversation_id
    }

    /// 返回上下文 Bean 列表。
    ///
    /// 返回借用切片以避免复制类型擦除对象。对应 Java:
    /// `ExecuteOption#getContextBeans`。
    #[must_use]
    pub fn get_context_beans(&self) -> &[(String, Arc<dyn Any + Send + Sync>)] {
        &self.context_beans
    }

    /// 返回事件监听器。对应 Java: `ExecuteOption#getEventListener`。
    #[must_use]
    pub fn get_event_listener(&self) -> Option<&Arc<dyn FlowEventListener>> {
        self.event_listener.as_ref()
    }

    /// 解析最终会话 ID（对应 `FlowExecutor` 内部 resolve 逻辑）。
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
