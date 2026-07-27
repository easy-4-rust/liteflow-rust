//! 对应 Java: `ReActAgentComponent` 的稳定组件能力定义。

use std::path::PathBuf;
use std::sync::Arc;

use agentscope_core::{
    AgentTool, Model, ReActAgent, Toolkit,
    hook::Hook,
    session::{Session, SessionKey},
};

use crate::hook::UsageTrackingModel;
use crate::skill::SkillLoadPermissionTool;
use crate::{
    AgentConfig, AgentError, ChatUsageTrackingHook, ManagedShellCommandTool, ReActLoggingHook,
    ShellMode, SkillBoxFactory, SkillTrackingHook, WorkspaceFileTools,
};

/// 构建并缓存 ReActAgent 所需的稳定定义。
pub struct AgentDefinition {
    agent_key: String,
    system_prompt: String,
    model: Arc<dyn Model>,
    tools: Vec<Arc<dyn AgentTool>>,
    hooks: Vec<Arc<dyn Hook>>,
    session: Option<Arc<dyn Session>>,
    config: AgentConfig,
    workspace_root: Option<PathBuf>,
    workspace_file_tools_enabled: bool,
    shell_tool_enabled: bool,
    react_logging_enabled: Option<bool>,
    skill_names: Vec<String>,
    skills_enabled: Option<bool>,
}

impl AgentDefinition {
    /// 创建 Agent 定义。
    ///
    /// 该构造器是组件构建器与不可变运行时定义之间的唯一装配边界，参数逐项对应
    /// Java `ReActAgentComponent#buildAgent` 中的模型、工具、会话和内置能力开关；
    /// 为避免再引入一个没有 Java 对应物的中转对象，保留显式参数。
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        agent_key: impl Into<String>,
        system_prompt: impl Into<String>,
        model: Arc<dyn Model>,
        tools: Vec<Arc<dyn AgentTool>>,
        hooks: Vec<Arc<dyn Hook>>,
        session: Option<Arc<dyn Session>>,
        config: AgentConfig,
        workspace_root: Option<PathBuf>,
        workspace_file_tools_enabled: bool,
        shell_tool_enabled: bool,
        react_logging_enabled: Option<bool>,
        skill_names: Vec<String>,
        skills_enabled: Option<bool>,
    ) -> Self {
        Self {
            agent_key: agent_key.into(),
            system_prompt: system_prompt.into(),
            model,
            tools,
            hooks,
            session,
            config,
            workspace_root,
            workspace_file_tools_enabled,
            shell_tool_enabled,
            react_logging_enabled,
            skill_names,
            skills_enabled,
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
    pub(crate) async fn build_agent(
        &self,
        conversation_id: &str,
    ) -> Result<
        (
            Arc<ReActAgent>,
            Option<Arc<SkillTrackingHook>>,
            Option<Arc<ReActLoggingHook>>,
            Arc<ChatUsageTrackingHook>,
            Option<PathBuf>,
        ),
        AgentError,
    > {
        let mut toolkit = Toolkit::new();
        let mut conversation_workspace = None;
        for tool in &self.tools {
            toolkit.register_agent_tool(Arc::clone(tool));
        }
        if let Some(workspace_root) = &self.workspace_root {
            let workspace = WorkspaceFileTools::for_conversation(
                workspace_root,
                conversation_id,
                &self.config.workspace,
            )?;
            if self.shell_tool_enabled && self.config.shell.mode != ShellMode::Disabled {
                toolkit.register_agent_tool(Arc::new(ManagedShellCommandTool::new(
                    workspace.workspace(),
                    &self.config,
                )));
            }
            conversation_workspace = Some(workspace.workspace().to_path_buf());
            if self.workspace_file_tools_enabled {
                for tool in workspace.tools() {
                    toolkit.register_agent_tool(tool);
                }
            }
        }
        let chat_usage_tracking_hook = Arc::new(ChatUsageTrackingHook::new());
        let tracked_model: Arc<dyn Model> = Arc::new(UsageTrackingModel::new(
            Arc::clone(&self.model),
            Arc::clone(&chat_usage_tracking_hook),
        ));
        let mut builder = ReActAgent::builder()
            .name(&self.agent_key)
            .sys_prompt(&self.system_prompt)
            .model_arc(tracked_model)
            .toolkit(toolkit)
            .max_iters(self.config.defaults.max_iterations)
            .session_key(SessionKey::new(conversation_id));
        let mut skill_tracking_hook = None;
        let mut react_logging_hook = None;
        let mut configured_skill_load_result = None;
        for hook in &self.hooks {
            builder = builder.hook_arc(Arc::clone(hook));
        }
        if self
            .react_logging_enabled
            .unwrap_or(self.config.logging.react_enabled)
        {
            let logging = Arc::new(ReActLoggingHook::new(format!(
                "{conversation_id}:{}",
                self.agent_key
            )));
            let hook: Arc<dyn Hook> = logging.clone();
            builder = builder.hook_arc(hook);
            react_logging_hook = Some(logging);
        }
        if self.skills_enabled.unwrap_or(self.config.skills.enabled) {
            let skill_load_result = SkillBoxFactory::build(
                &self.config,
                &self.skill_names,
                conversation_workspace.as_deref(),
            )
            .await?;
            let hook = Arc::new(SkillTrackingHook::new(
                skill_load_result.skill_id_to_name().clone(),
            ));
            builder = builder
                .hook_arc(Arc::clone(&hook) as Arc<dyn Hook>)
                .skill_box(Arc::clone(skill_load_result.skill_box()));
            skill_tracking_hook = Some(hook);
            configured_skill_load_result = Some(skill_load_result);
        }
        if let Some(session) = &self.session {
            builder = builder.session_arc(Arc::clone(session));
        }
        // AgentScope 的 Middleware 洋葱链依赖 ReActAgent 的自引用 Arc；
        // 直接 build 后再外包 Arc 会静默跳过全部 Middleware。
        let agent = ReActAgent::build_arc(builder);
        if let Some(skill_load_result) = configured_skill_load_result {
            let skill_box = skill_load_result.skill_box();
            let toolkit = skill_box.toolkit().ok_or_else(|| {
                AgentError::SkillsLoad("SkillBox toolkit is not bound".to_string())
            })?;

            // 每个技能的工具放入初始关闭的专属工具组；加载技能时 AgentScope
            // 会按 skill ID/名称激活该组，与 Java SkillBox.registration() 一致。
            for (skill_id, tools) in skill_load_result.skill_tools() {
                let skill = skill_box.get(skill_id).ok_or_else(|| {
                    AgentError::SkillsLoad(format!(
                        "Skill '{skill_id}' disappeared before tool registration"
                    ))
                })?;
                let group_name = format!("{skill_id}_skill_tools");
                let mut toolkit = toolkit.write();
                toolkit.create_skill_tool_group(
                    &group_name,
                    &format!("Tools declared by skill '{}'", skill.name),
                    false,
                    &skill.name,
                );
                for tool in tools {
                    toolkit
                        .registration()
                        .agent_tool(Arc::clone(tool))
                        .group(&group_name)
                        .apply();
                }
            }
            let delegate = toolkit
                .read()
                .get_tool(SkillTrackingHook::LOAD_SKILL_TOOL_NAME)
                .ok_or_else(|| {
                    AgentError::SkillsLoad(
                        "AgentScope skill load tool was not registered".to_string(),
                    )
                })?;
            toolkit
                .write()
                .register_agent_tool(Arc::new(SkillLoadPermissionTool::new(delegate)));
        }
        Ok((
            agent,
            skill_tracking_hook,
            react_logging_hook,
            chat_usage_tracking_hook,
            conversation_workspace,
        ))
    }
}
