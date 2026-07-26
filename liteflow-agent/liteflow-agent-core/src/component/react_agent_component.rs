//! 对应 Java: `com.yomahub.liteflow.agent.component.ReActAgentComponent`。

use std::sync::Arc;

use agentscope_core::Agent;
use async_trait::async_trait;
use liteflow_core::{CmpContext, FlowEvent, LiteflowError, core::NodeComponent};
use serde_json::{Value, json};

use super::SharedPromptResolver;
use crate::{AgentDefinition, AgentError, AgentEventType, AgentSessionManager};

/// 将 AgentScope ReActAgent 暴露为普通 LiteFlow NodeComponent。
pub struct ReActAgentComponent {
    node_id: String,
    definition: Arc<AgentDefinition>,
    sessions: AgentSessionManager,
    prompt_resolver: SharedPromptResolver,
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
    ) -> Self {
        Self {
            node_id,
            sessions: AgentSessionManager::new(Arc::clone(&definition)),
            definition,
            prompt_resolver,
        }
    }

    /// 返回会话管理器。
    #[must_use]
    pub fn sessions(&self) -> &AgentSessionManager {
        &self.sessions
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

    fn publish(&self, context: &CmpContext, event_type: &str, text: String, last: bool) {
        if !self.definition.config().publish_events {
            return;
        }
        let mut event = FlowEvent::builder(event_type)
            .chain_id(context.chain_id())
            .node_id(context.node_id())
            .request_id(context.request_id())
            .text(text)
            .last(last);
        if let Some(conversation_id) = context.conversation_id() {
            event = event.conversation_id(conversation_id);
        }
        context.publish_event(&event.build());
    }
}

#[async_trait]
impl NodeComponent for ReActAgentComponent {
    async fn process(&self, context: &CmpContext) -> Result<Value, LiteflowError> {
        let prompt = (self.prompt_resolver)(context)?;
        if prompt.trim().is_empty() {
            return Err(agent_error(context, AgentError::BlankUserPrompt));
        }
        let conversation_id = self.conversation_id(context);
        let session = self.sessions.get_or_create(&conversation_id);
        let _guard = session.gate().lock().await;

        self.publish(
            context,
            AgentEventType::REASONING,
            "AgentScope ReAct execution started".to_string(),
            false,
        );
        let message = session
            .agent()
            .call_with_text(&prompt)
            .await
            .map_err(|error| agent_error(context, AgentError::Execution(error.to_string())))?;
        let text = message.get_text_content();
        context.set_data(
            self.definition.config().result_key.clone(),
            json!(text.clone()),
        );
        self.publish(context, AgentEventType::RESULT, text.clone(), true);
        Ok(Value::String(text))
    }

    fn name(&self) -> &str {
        &self.node_id
    }
}

fn agent_error(context: &CmpContext, error: AgentError) -> LiteflowError {
    LiteflowError::NodeExec {
        node: context.node_id().to_string(),
        msg: error.to_string(),
        kind: "AgentError".to_string(),
    }
}
