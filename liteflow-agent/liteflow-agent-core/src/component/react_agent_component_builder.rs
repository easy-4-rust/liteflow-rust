//! 对应 Java `ReActAgentComponent` 的 Rust 构建器。

use std::sync::Arc;

use agentscope_core::{
    AgentTool, Model,
    session::{InMemorySession, JsonSession, Session},
};
use liteflow_core::{CmpContext, LFResult};
use serde_json::Value;

use super::SharedPromptResolver;
use crate::{
    AgentConfig, AgentDefinition, AgentError, LocalFileMemoryConfig, MemoryStorageMode,
    ReActAgentComponent,
};

/// 组装模型、工具、Session、提示词与 LiteFlow 节点身份。
pub struct ReActAgentComponentBuilder {
    node_id: String,
    agent_key: String,
    system_prompt: String,
    model: Arc<dyn Model>,
    tools: Vec<Arc<dyn AgentTool>>,
    session: Option<Arc<dyn Session>>,
    session_explicit: bool,
    config: AgentConfig,
    prompt_resolver: SharedPromptResolver,
}

impl ReActAgentComponentBuilder {
    /// 创建构建器。默认 agentKey 等于 nodeId，Session 为进程内存。
    pub fn new(node_id: impl Into<String>, model: Arc<dyn Model>) -> Self {
        let node_id = node_id.into();
        Self {
            agent_key: node_id.clone(),
            node_id,
            system_prompt: "请使用用户提问所用的语言回答；只输出可公开的简短推理摘要。".to_string(),
            model,
            tools: Vec::new(),
            session: Some(Arc::new(InMemorySession::new())),
            session_explicit: false,
            config: AgentConfig::default(),
            prompt_resolver: Arc::new(default_prompt),
        }
    }

    /// 设置稳定 agentKey。
    #[must_use]
    pub fn agent_key(mut self, agent_key: impl Into<String>) -> Self {
        self.agent_key = agent_key.into();
        self
    }

    /// 设置系统提示词。
    #[must_use]
    pub fn system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.system_prompt = system_prompt.into();
        self
    }

    /// 设置 Agent 配置。
    #[must_use]
    pub fn config(mut self, config: AgentConfig) -> Self {
        self.config = config;
        self
    }

    /// 注册 AgentScope 工具。
    #[must_use]
    pub fn tool(mut self, tool: Arc<dyn AgentTool>) -> Self {
        self.tools.push(tool);
        self
    }

    /// 设置 AgentScope Session 实现。
    #[must_use]
    pub fn session(mut self, session: Arc<dyn Session>) -> Self {
        self.session = Some(session);
        self.session_explicit = true;
        self
    }

    /// 禁用持久 Session；ReActAgent 实例缓存仍保持同一会话的进程内上下文。
    #[must_use]
    pub fn without_session(mut self) -> Self {
        self.session = None;
        self.session_explicit = true;
        self
    }

    /// 设置每次执行时的用户提示词解析函数。
    #[must_use]
    pub fn user_prompt<F>(mut self, resolver: F) -> Self
    where
        F: Fn(&CmpContext) -> LFResult<String> + Send + Sync + 'static,
    {
        self.prompt_resolver = Arc::new(resolver);
        self
    }

    /// 完成组件构建。
    pub fn build(self) -> Result<ReActAgentComponent, AgentError> {
        if self.agent_key.trim().is_empty() {
            return Err(AgentError::BlankAgentKey);
        }
        let session = self.resolve_session()?;
        let definition = Arc::new(AgentDefinition::new(
            self.agent_key,
            self.system_prompt,
            self.model,
            self.tools,
            session,
            self.config,
        ));
        Ok(ReActAgentComponent::new(
            self.node_id,
            definition,
            self.prompt_resolver,
        ))
    }

    fn resolve_session(&self) -> Result<Option<Arc<dyn Session>>, AgentError> {
        if self.session_explicit {
            return Ok(self.session.clone());
        }

        // Java 由 AgentSessionFactoryRegistry 根据 mode 选择后端；Rust 直接构造
        // AgentScope Session。需要宿主客户端的 Redis/MySQL 必须显式注入。
        match self.config.session.memory.mode {
            MemoryStorageMode::None => Ok(None),
            MemoryStorageMode::Jvm => Ok(Some(Arc::new(InMemorySession::new()))),
            MemoryStorageMode::LocalFile => {
                let root = self
                    .config
                    .workspace
                    .root
                    .as_ref()
                    .ok_or(AgentError::WorkspaceRootRequired)?;
                let directory = std::path::PathBuf::from(root).join(LocalFileMemoryConfig::SUB_DIR);
                Ok(Some(Arc::new(JsonSession::with_dir(directory))))
            }
            mode @ (MemoryStorageMode::Redis | MemoryStorageMode::Mysql) => {
                Err(AgentError::SessionBackendRequiresInjection(mode))
            }
        }
    }
}

fn default_prompt(context: &CmpContext) -> LFResult<String> {
    let input = context.request_data::<Value>().unwrap_or(Value::Null);
    Ok(match input {
        Value::String(prompt) => prompt,
        Value::Object(object) => object
            .get("prompt")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| Value::Object(object).to_string()),
        other => other.to_string(),
    })
}
