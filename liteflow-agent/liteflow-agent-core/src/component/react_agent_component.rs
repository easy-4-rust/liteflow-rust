//! 对应 Java: `com.yomahub.liteflow.agent.component.ReActAgentComponent`。

use std::sync::Arc;

use agentscope_core::agent::StreamOptions;
use agentscope_core::agent::event_type::EventType;
use agentscope_core::event::AgentEvent;
use agentscope_core::{Agent, Msg, MsgRole, ReActAgent};
use async_trait::async_trait;
use futures::StreamExt;
use liteflow_core::{CmpContext, FlowEvent, LiteflowError, core::NodeComponent};
use serde_json::{Value, json};

use super::{ReActAgentContext, SharedPromptResolver, SharedReplyHandler};
use crate::{AgentDefinition, AgentError, AgentEventType, AgentSessionManager};

/// 将 AgentScope ReActAgent 暴露为普通 LiteFlow NodeComponent。
pub struct ReActAgentComponent {
    node_id: String,
    definition: Arc<AgentDefinition>,
    sessions: AgentSessionManager,
    prompt_resolver: SharedPromptResolver,
    reply_handler: Option<SharedReplyHandler>,
}

/// 以 RAII 对齐 Java `process` 中上下文附件的 `try/finally` 清理语义。
///
/// 该内部伴随对象随组件调用 Future 一同持有；即使 Future 被取消或 drop，也会移除
/// invocation 上下文，避免 Slot 池复用后残留悬挂附件。
struct RuntimeContextAttachment {
    context: CmpContext,
    key: String,
}

impl RuntimeContextAttachment {
    fn bind(context: CmpContext, key: String, runtime_context: ReActAgentContext) -> Self {
        context.set_attachment(key.clone(), runtime_context);
        Self { context, key }
    }
}

impl Drop for RuntimeContextAttachment {
    fn drop(&mut self) {
        self.context.remove_attachment(&self.key);
    }
}

impl ReActAgentComponent {
    /// 创建构建器。
    pub fn builder(
        node_id: impl Into<String>,
        model: Arc<dyn agentscope_core::Model>,
    ) -> crate::ReActAgentComponentBuilder {
        crate::ReActAgentComponentBuilder::new(node_id, model)
    }

    pub(crate) fn new(
        node_id: String,
        definition: Arc<AgentDefinition>,
        prompt_resolver: SharedPromptResolver,
        reply_handler: Option<SharedReplyHandler>,
    ) -> Self {
        Self {
            node_id,
            sessions: AgentSessionManager::new(Arc::clone(&definition)),
            definition,
            prompt_resolver,
            reply_handler,
        }
    }

    /// 返回会话管理器。
    #[must_use]
    pub fn sessions(&self) -> &AgentSessionManager {
        &self.sessions
    }

    /// 返回当前节点 invocation 已绑定的 Agent 运行时上下文。
    ///
    /// 仅在用户提示词解析、Agent 调用及 `handle_reply` 回调期间返回 `Some`；调用结束
    /// 或失败后框架都会移除附件，避免缓存对象持有已失效的 LiteFlow 上下文。
    ///
    /// 对应 Java: `ReActAgentComponent#ctx`。
    #[must_use]
    pub fn runtime_context(&self, context: &CmpContext) -> Option<Arc<ReActAgentContext>> {
        context.get_attachment(&self.context_attachment_key())
    }

    fn conversation_id(&self, context: &CmpContext) -> String {
        context
            .conversation_id()
            .map(ToOwned::to_owned)
            .or_else(|| {
                context
                    .request_data::<Value>()
                    .and_then(|input| input.get("conversationId")?.as_str().map(ToOwned::to_owned))
            })
            .unwrap_or_else(|| context.request_id().to_string())
    }

    fn context_attachment_key(&self) -> String {
        format!("liteflow.agent.context.{}", self.node_id)
    }

    fn publish(
        &self,
        context: &CmpContext,
        event_type: &str,
        text: Option<String>,
        last: bool,
        data: Option<Value>,
    ) {
        if !self.definition.config().publish_events {
            return;
        }
        let mut event = FlowEvent::builder()
            .r#type(event_type)
            .chain_id(context.chain_id())
            .node_id(context.node_id())
            .request_id(context.request_id())
            .last(last);
        if let Some(text) = text {
            event = event.text(text);
        }
        if let Some(data) = data {
            event = event.data(data);
        }
        if let Some(conversation_id) = context.conversation_id() {
            event = event.conversation_id(conversation_id);
        }
        context.publish_event(&event.build());
    }

    fn has_event_listener(&self, context: &CmpContext) -> bool {
        self.definition.config().publish_events
            && context.has_attachment(liteflow_core::flow::flow_event_publisher::LISTENER_KEY)
    }

    async fn stream_agent(
        &self,
        context: &CmpContext,
        agent: &Arc<ReActAgent>,
        prompt: &str,
    ) -> Result<Msg, AgentError> {
        let user_message = Msg::builder()
            .role(MsgRole::User)
            .text_content(prompt)
            .build();
        let options = StreamOptions::builder()
            .event_types([
                EventType::Reasoning,
                EventType::ToolResult,
                EventType::Summary,
                EventType::AgentResult,
            ])
            .incremental(true)
            .build();
        let mut events = agent.stream_events_with_options(vec![user_message], Some(options));
        let mut final_message = None;
        let mut summary_mode = false;
        let mut stop_reason = None;

        while let Some(event) = events.next().await {
            if let AgentEvent::RequestStop(value) = &event {
                stop_reason = Some(value.reason.clone());
            }
            if let Some((event_type, text, last)) = flow_event_projection(&event, &mut summary_mode)
            {
                let data = serde_json::to_value(&event).ok();
                self.publish(context, event_type, text, last, data);
            }
            if let AgentEvent::AgentResult(value) = event {
                final_message = Some(value.result);
            }
        }

        final_message.ok_or_else(|| {
            AgentError::Execution(stop_reason.unwrap_or_else(|| {
                "AgentScope event stream completed without an AgentResult event".to_string()
            }))
        })
    }
}

#[async_trait]
impl NodeComponent for ReActAgentComponent {
    async fn process(&self, context: &CmpContext) -> Result<Value, LiteflowError> {
        let conversation_id = self.conversation_id(context);
        let session = self
            .sessions
            .get_or_create(&conversation_id)
            .await
            .map_err(|error| agent_error(context, error))?;
        let _guard = session.gate().lock().await;
        session.clear_used_skills();
        session.reset_chat_usage();

        // Java 在 Slot 中绑定 per-invocation ReActAgentContext。Rust 使用同一附件机制，
        // 并把清理放在统一出口，确保提示词、模型或回复处理任一阶段失败都不会泄漏上下文。
        let runtime_context = ReActAgentContext::new(
            context.clone(),
            &conversation_id,
            self.definition.agent_key(),
            session.workspace_dir().map(ToOwned::to_owned),
            session.chat_usage_tracking_hook(),
        );
        let attachment_key = self.context_attachment_key();
        let _context_attachment = RuntimeContextAttachment::bind(
            context.clone(),
            attachment_key,
            runtime_context.clone(),
        );

        async {
            let prompt = (self.prompt_resolver)(context)?;
            if prompt.trim().is_empty() {
                return Err(agent_error(context, AgentError::BlankUserPrompt));
            }

            session.log_reasoning_start(1);
            let streamed = self.has_event_listener(context);
            if !streamed {
                self.publish(
                    context,
                    AgentEventType::REASONING,
                    Some("AgentScope ReAct execution started".to_string()),
                    false,
                    None,
                );
            }
            let call_result = if streamed {
                self.stream_agent(context, session.agent(), &prompt).await
            } else {
                session
                    .agent()
                    .call_with_text(&prompt)
                    .await
                    .map_err(|error| AgentError::Execution(error.to_string()))
            };
            let message = match call_result {
                Ok(message) => message,
                Err(error) => {
                    session.log_execution_error(&error.to_string());
                    return Err(agent_error(context, error));
                }
            };
            session.log_final_message(&message);
            let text = message.get_text_content();
            context.set_data(
                self.definition.config().result_key.clone(),
                json!(text.clone()),
            );
            if let Some(handler) = &self.reply_handler {
                handler(&runtime_context, &message)?;
            }
            if !streamed {
                self.publish(
                    context,
                    AgentEventType::RESULT,
                    Some(text.clone()),
                    true,
                    None,
                );
            }
            Ok(Value::String(text))
        }
        .await
    }

    fn name(&self) -> &str {
        &self.node_id
    }
}

fn flow_event_projection(
    event: &AgentEvent,
    summary_mode: &mut bool,
) -> Option<(&'static str, Option<String>, bool)> {
    match event {
        AgentEvent::AgentResult(value) => Some((
            AgentEventType::RESULT,
            Some(value.result.get_text_content()),
            true,
        )),
        AgentEvent::ToolResultTextDelta(value) => Some((
            AgentEventType::TOOL_RESULT,
            Some(value.delta.clone()),
            false,
        )),
        AgentEvent::ToolResultDataDelta(value) => Some((
            AgentEventType::TOOL_RESULT,
            serde_json::to_string(&value.data).ok(),
            false,
        )),
        AgentEvent::ToolResultStart(_) => Some((AgentEventType::TOOL_RESULT, None, false)),
        AgentEvent::ToolResultEnd(_) | AgentEvent::AllToolsDenied(_) => {
            Some((AgentEventType::TOOL_RESULT, None, true))
        }
        AgentEvent::ExceedMaxIters(value) => {
            *summary_mode = true;
            Some((
                AgentEventType::SUMMARY,
                Some(format!(
                    "Maximum iterations reached: current={}, max={}",
                    value.current_iter, value.max_iters
                )),
                false,
            ))
        }
        AgentEvent::TextBlockDelta(value) => Some((
            reasoning_or_summary(*summary_mode),
            Some(value.delta.clone()),
            false,
        )),
        AgentEvent::ThinkingBlockDelta(value) => Some((
            reasoning_or_summary(*summary_mode),
            Some(value.delta.clone()),
            false,
        )),
        AgentEvent::DataBlockDelta(value) => Some((
            reasoning_or_summary(*summary_mode),
            Some(value.delta.clone()),
            false,
        )),
        AgentEvent::ToolCallDelta(value) => Some((
            reasoning_or_summary(*summary_mode),
            Some(value.delta.clone()),
            false,
        )),
        AgentEvent::ModelCallStart(_)
        | AgentEvent::TextBlockStart(_)
        | AgentEvent::ThinkingBlockStart(_)
        | AgentEvent::DataBlockStart(_)
        | AgentEvent::ToolCallStart(_) => Some((reasoning_or_summary(*summary_mode), None, false)),
        AgentEvent::ModelCallEnd(_)
        | AgentEvent::TextBlockEnd(_)
        | AgentEvent::ThinkingBlockEnd(_)
        | AgentEvent::DataBlockEnd(_)
        | AgentEvent::ToolCallEnd(_) => Some((reasoning_or_summary(*summary_mode), None, true)),
        _ => None,
    }
}

fn reasoning_or_summary(summary_mode: bool) -> &'static str {
    if summary_mode {
        AgentEventType::SUMMARY
    } else {
        AgentEventType::REASONING
    }
}

fn agent_error(context: &CmpContext, error: AgentError) -> LiteflowError {
    LiteflowError::NodeExec {
        node: context.node_id().to_string(),
        msg: error.to_string(),
        kind: "AgentError".to_string(),
        code: None,
    }
}
