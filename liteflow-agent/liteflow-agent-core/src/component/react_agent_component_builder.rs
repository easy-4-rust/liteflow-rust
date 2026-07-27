//! 对应 Java `ReActAgentComponent` 的 Rust 构建器。

use std::sync::Arc;

use agentscope_core::{AgentTool, Model, Msg, hook::Hook, session::Session};
use liteflow_core::{CmpContext, LFResult};
use serde_json::Value;

use super::{ReActAgentContext, SharedPromptResolver, SharedReplyHandler};
use crate::{
    AgentConfig, AgentDefinition, AgentError, AgentSessionFactoryRegistry, ReActAgentComponent,
    WorkspaceFileTools,
};

/// 组装模型、工具、Session、提示词与 LiteFlow 节点身份。
pub struct ReActAgentComponentBuilder {
    node_id: String,
    agent_key: String,
    system_prompt: String,
    model: Arc<dyn Model>,
    tools: Vec<Arc<dyn AgentTool>>,
    hooks: Vec<Arc<dyn Hook>>,
    session: Option<Arc<dyn Session>>,
    session_explicit: bool,
    shell_tool_enabled: bool,
    workspace_file_tools_enabled: bool,
    react_logging_enabled: Option<bool>,
    skill_names: Vec<String>,
    skills_enabled: Option<bool>,
    config: AgentConfig,
    prompt_resolver: SharedPromptResolver,
    reply_handler: Option<SharedReplyHandler>,
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
            hooks: Vec::new(),
            session: None,
            session_explicit: false,
            shell_tool_enabled: true,
            workspace_file_tools_enabled: true,
            react_logging_enabled: None,
            skill_names: Vec::new(),
            skills_enabled: None,
            config: AgentConfig::default(),
            prompt_resolver: Arc::new(default_prompt),
            reply_handler: None,
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

    /// 注册自定义 AgentScope 生命周期 Hook。
    ///
    /// # 参数
    /// - `hook`: 在内置日志 Hook 之前注册的共享 Hook。
    ///
    /// # 返回
    /// 更新后的构建器。
    ///
    /// 对应 Java: `ReActAgentComponent#hooks` 扩展点。
    #[must_use]
    pub fn hook(mut self, hook: Arc<dyn Hook>) -> Self {
        self.hooks.push(hook);
        self
    }

    /// 覆盖 ReAct 内部事件日志开关。
    ///
    /// # 参数
    /// - `enabled`: 是否注册 `ReActLoggingHook`。
    ///
    /// # 返回
    /// 更新后的构建器；未调用时读取 `AgentConfig.logging.react_enabled`。
    ///
    /// 对应 Java: `ReActAgentComponent#enableReActLogging`。
    #[must_use]
    pub fn enable_react_logging(mut self, enabled: bool) -> Self {
        self.react_logging_enabled = Some(enabled);
        self
    }

    /// 声明当前 Agent 允许加载的技能名称。
    ///
    /// # 参数
    /// - `skill_names`: 与 `SKILL.md` 元数据中 `name` 对应的名称序列；空序列表示全部。
    ///
    /// # 返回
    /// 更新后的构建器，名称顺序会作为筛选和跟踪顺序。
    ///
    /// 对应 Java: `ReActAgentComponent#skills`。
    #[must_use]
    pub fn skills<I, S>(mut self, skill_names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.skill_names = skill_names.into_iter().map(Into::into).collect();
        self
    }

    /// 覆盖配置驱动的 Skills 开关。
    ///
    /// # 参数
    /// - `enabled`: 是否为当前 Agent 构造并绑定 AgentScope SkillBox。
    ///
    /// # 返回
    /// 更新后的构建器；未调用时读取 `AgentConfig.skills.enabled`。
    ///
    /// 对应 Java: `ReActAgentComponent#enableSkills`。
    #[must_use]
    pub fn enable_skills(mut self, enabled: bool) -> Self {
        self.skills_enabled = Some(enabled);
        self
    }

    /// 设置是否自动注册工作区文件工具。
    ///
    /// # 参数
    /// - `enabled`: `true` 时，在配置了工作区根目录后为每个会话注册四个文件工具。
    ///
    /// # 返回
    /// 更新后的构建器。
    ///
    /// 对应 Java: `ReActAgentComponent#enableWorkspaceFileTools`。
    #[must_use]
    pub fn enable_workspace_file_tools(mut self, enabled: bool) -> Self {
        self.workspace_file_tools_enabled = enabled;
        self
    }

    /// 设置是否自动注册受控 Shell 工具。
    ///
    /// # 参数
    /// - `enabled`: `true` 时，在配置工作区且 Shell 模式非 `DISABLED` 后注册工具。
    ///
    /// # 返回
    /// 更新后的构建器。
    ///
    /// 对应 Java: `ReActAgentComponent#enableShellTool`。
    #[must_use]
    pub fn enable_shell_tool(mut self, enabled: bool) -> Self {
        self.shell_tool_enabled = enabled;
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

    /// 设置 Agent 返回消息处理扩展点。
    ///
    /// 回调在模型完成全部 ReAct 步骤、usage 已累计完成后执行，因此可通过
    /// `ReActAgentContext::chat_usage` 读取本次调用的完整用量。框架会先把文本写入
    /// 配置的结果 key；回调用于追加业务处理，不需要重复默认行为。
    ///
    /// # 参数
    /// - `handler`: 接收当前 invocation 上下文与 AgentScope 最终消息的线程安全函数。
    ///
    /// # 返回
    /// 更新后的构建器。
    ///
    /// 对应 Java: `ReActAgentComponent#handleReply`。
    #[must_use]
    pub fn handle_reply<F>(mut self, handler: F) -> Self
    where
        F: Fn(&ReActAgentContext, &Msg) -> LFResult<()> + Send + Sync + 'static,
    {
        self.reply_handler = Some(Arc::new(handler));
        self
    }

    /// 完成组件构建。
    pub fn build(self) -> Result<ReActAgentComponent, AgentError> {
        if self.agent_key.trim().is_empty() {
            return Err(AgentError::BlankAgentKey);
        }
        let session = self.resolve_session()?;
        let skills_enabled = self.skills_enabled.unwrap_or(self.config.skills.enabled);
        let needs_workspace = self.workspace_file_tools_enabled
            || (self.shell_tool_enabled && self.config.shell.mode != crate::ShellMode::Disabled)
            || skills_enabled;
        let workspace_root = if needs_workspace {
            WorkspaceFileTools::prepare_root(&self.config.workspace)?
        } else {
            None
        };
        let definition = Arc::new(AgentDefinition::new(
            self.agent_key,
            self.system_prompt,
            self.model,
            self.tools,
            self.hooks,
            session,
            self.config,
            workspace_root,
            self.workspace_file_tools_enabled,
            self.shell_tool_enabled,
            self.react_logging_enabled,
            self.skill_names,
            self.skills_enabled,
        ));
        Ok(ReActAgentComponent::new(
            self.node_id,
            definition,
            self.prompt_resolver,
            self.reply_handler,
        ))
    }

    fn resolve_session(&self) -> Result<Option<Arc<dyn Session>>, AgentError> {
        if self.session_explicit {
            return Ok(self.session.clone());
        }

        let mode = self.config.session.memory.mode;
        AgentSessionFactoryRegistry::new()
            .create_session(&self.config)
            .map_err(|error| match mode {
                crate::MemoryStorageMode::Redis | crate::MemoryStorageMode::Mysql => {
                    AgentError::SessionBackendRequiresInjection(mode)
                }
                _ => AgentError::from(error),
            })
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
