//! 对应 Java: `ReActAgentComponent` 的稳定组件能力定义。

use std::sync::Arc;

use agentscope_core::{
    AgentTool, Model, ReActAgent, Toolkit,
    session::{Session, SessionKey},
};

use crate::AgentConfig;

/// 构建并缓存 ReActAgent 所需的稳定定义。
pub struct AgentDefinition {
    agent_key: String,
    system_prompt: String,
    model: Arc<dyn Model>,
    tools: Vec<Arc<dyn AgentTool>>,
    session: Option<Arc<dyn Session>>,
    config: AgentConfig,
}

impl AgentDefinition {
    /// 创建 Agent 定义。
    pub fn new(
        agent_key: impl Into<String>,
        system_prompt: impl Into<String>,
        model: Arc<dyn Model>,
        tools: Vec<Arc<dyn AgentTool>>,
        session: Option<Arc<dyn Session>>,
        config: AgentConfig,
    ) -> Self {
        Self {
            agent_key: agent_key.into(),
            system_prompt: system_prompt.into(),
            model,
            tools,
            session,
            config,
        }
    }

    /// 返回稳定 agent key。
    #[must_use]
    pub fn agent_key(&self) -> &str {
        &self.agent_key
    }

    /// 返回配置。
    #[must_use]
    pub fn config(&self) -> &AgentConfig {
        &self.config
    }

    /// 为指定 conversation 构建 AgentScope ReActAgent。
    pub(crate) fn build_agent(&self, conversation_id: &str) -> ReActAgent {
        let mut toolkit = Toolkit::new();
        for tool in &self.tools {
            toolkit.register_agent_tool(Arc::clone(tool));
        }
        let mut builder = ReActAgent::builder()
            .name(&self.agent_key)
            .sys_prompt(&self.system_prompt)
            .model_arc(Arc::clone(&self.model))
            .toolkit(toolkit)
            .max_iters(self.config.defaults.max_iterations)
            .session_key(SessionKey::new(conversation_id));
        if let Some(session) = &self.session {
            builder = builder.session_arc(Arc::clone(session));
        }
        builder.build()
    }
}
