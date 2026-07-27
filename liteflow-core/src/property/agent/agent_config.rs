use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{
    DefaultsConfig, LoggingConfig, PlatformCredential, SessionConfig, ShellConfig, SkillsConfig,
    WorkspaceConfig,
};

/// ReAct Agent 模块的根配置对象。
///
/// 对应 Spring Boot 配置段 `liteflow.agent.*`，聚合工作区、会话、Shell、默认值、
/// 日志、Skills 和模型平台凭证。Jackson/Spring 属性绑定在 Rust 中由 serde 完成。
///
/// 对应 Java: `com.yomahub.liteflow.property.agent.AgentConfig`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AgentConfig {
    /// 工作区配置，控制会话工作目录、创建和清理策略。
    pub workspace: WorkspaceConfig,
    /// 会话配置，控制缓存生命周期和记忆持久化。
    pub session: SessionConfig,
    /// Shell 工具安全配置。
    pub shell: ShellConfig,
    /// ReAct 全局默认值。
    pub defaults: DefaultsConfig,
    /// ReAct 内部事件日志开关。
    pub logging: LoggingConfig,
    /// AgentScope Skills 加载配置。
    pub skills: SkillsConfig,
    /// OpenAI 头等平台凭证。
    pub openai: PlatformCredential,
    /// Anthropic 头等平台凭证。
    pub anthropic: PlatformCredential,
    /// Gemini 头等平台凭证。
    pub gemini: PlatformCredential,
    /// DashScope 头等平台凭证。
    pub dashscope: PlatformCredential,
    /// OpenAI 兼容平台凭证集合。
    pub openai_compatible: HashMap<String, PlatformCredential>,
    /// Anthropic 兼容平台凭证集合。
    pub anthropic_compatible: HashMap<String, PlatformCredential>,
    /// Rust 扩展：是否把 Agent 生命周期事件发布为 LiteFlow FlowEvent。
    pub publish_events: bool,
    /// Rust 扩展：最终文本写入 Slot data 的键。
    pub result_key: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            workspace: WorkspaceConfig::default(),
            session: SessionConfig::default(),
            shell: ShellConfig::default(),
            defaults: DefaultsConfig::default(),
            logging: LoggingConfig::default(),
            skills: SkillsConfig::default(),
            openai: PlatformCredential::default(),
            anthropic: PlatformCredential::default(),
            gemini: PlatformCredential::default(),
            dashscope: PlatformCredential::default(),
            openai_compatible: HashMap::new(),
            anthropic_compatible: HashMap::new(),
            publish_events: true,
            result_key: "agent_result".to_string(),
        }
    }
}

impl AgentConfig {
    /// 返回工作区配置。对应 Java: `AgentConfig#getWorkspace`。
    #[must_use]
    pub fn workspace(&self) -> &WorkspaceConfig {
        &self.workspace
    }

    /// 返回工作区配置。
    ///
    /// 返回值控制会话根目录、自动创建、过期清理和文件大小上限。对应 Java:
    /// `AgentConfig#getWorkspace`。
    #[must_use]
    pub fn get_workspace(&self) -> &WorkspaceConfig {
        self.workspace()
    }

    /// 设置工作区配置。对应 Java: `AgentConfig#setWorkspace`。
    pub fn set_workspace(&mut self, workspace: WorkspaceConfig) {
        self.workspace = workspace;
    }

    /// 返回会话配置。对应 Java: `AgentConfig#getSession`。
    #[must_use]
    pub fn session(&self) -> &SessionConfig {
        &self.session
    }

    /// 返回 Agent 会话生命周期配置。
    ///
    /// 返回值供会话管理器读取空闲超时、清理周期、并发上限和记忆持久化方式。
    /// 对应 Java: `AgentConfig#getSession`。
    #[must_use]
    pub fn get_session(&self) -> &SessionConfig {
        self.session()
    }

    /// 设置会话配置。对应 Java: `AgentConfig#setSession`。
    pub fn set_session(&mut self, session: SessionConfig) {
        self.session = session;
    }

    /// 返回 Shell 配置。对应 Java: `AgentConfig#getShell`。
    #[must_use]
    pub fn shell(&self) -> &ShellConfig {
        &self.shell
    }

    /// 返回 Shell 工具安全配置。
    ///
    /// 返回值决定命令过滤模式、执行超时和输出截断。对应 Java:
    /// `AgentConfig#getShell`。
    #[must_use]
    pub fn get_shell(&self) -> &ShellConfig {
        self.shell()
    }

    /// 设置 Shell 配置。对应 Java: `AgentConfig#setShell`。
    pub fn set_shell(&mut self, shell: ShellConfig) {
        self.shell = shell;
    }

    /// 返回全局默认值。对应 Java: `AgentConfig#getDefaults`。
    #[must_use]
    pub fn defaults(&self) -> &DefaultsConfig {
        &self.defaults
    }

    /// 返回 ReAct Agent 全局默认值。
    ///
    /// 组件未显式配置最大迭代次数时使用此对象。对应 Java:
    /// `AgentConfig#getDefaults`。
    #[must_use]
    pub fn get_defaults(&self) -> &DefaultsConfig {
        self.defaults()
    }

    /// 设置全局默认值。对应 Java: `AgentConfig#setDefaults`。
    pub fn set_defaults(&mut self, defaults: DefaultsConfig) {
        self.defaults = defaults;
    }

    /// 返回日志配置。对应 Java: `AgentConfig#getLogging`。
    #[must_use]
    pub fn logging(&self) -> &LoggingConfig {
        &self.logging
    }

    /// 返回 ReAct 内部事件日志配置。
    ///
    /// 返回值控制 reason、act、error 等事件是否输出。对应 Java:
    /// `AgentConfig#getLogging`。
    #[must_use]
    pub fn get_logging(&self) -> &LoggingConfig {
        self.logging()
    }

    /// 设置日志配置。对应 Java: `AgentConfig#setLogging`。
    pub fn set_logging(&mut self, logging: LoggingConfig) {
        self.logging = logging;
    }

    /// 返回 Skills 配置。对应 Java: `AgentConfig#getSkills`。
    #[must_use]
    pub fn skills(&self) -> &SkillsConfig {
        &self.skills
    }

    /// 返回 AgentScope Skills 加载配置。
    ///
    /// 返回值控制技能目录、启用状态和严格解析模式。对应 Java:
    /// `AgentConfig#getSkills`。
    #[must_use]
    pub fn get_skills(&self) -> &SkillsConfig {
        self.skills()
    }

    /// 设置 Skills 配置。对应 Java: `AgentConfig#setSkills`。
    pub fn set_skills(&mut self, skills: SkillsConfig) {
        self.skills = skills;
    }

    /// 返回 OpenAI 凭证。对应 Java: `AgentConfig#getOpenai`。
    #[must_use]
    pub fn openai(&self) -> &PlatformCredential {
        &self.openai
    }

    /// 返回 OpenAI 头等平台凭证。
    ///
    /// 返回值由 OpenAI ProviderSpec 解析使用。对应 Java:
    /// `AgentConfig#getOpenai`。
    #[must_use]
    pub fn get_openai(&self) -> &PlatformCredential {
        self.openai()
    }

    /// 设置 OpenAI 凭证。对应 Java: `AgentConfig#setOpenai`。
    pub fn set_openai(&mut self, openai: PlatformCredential) {
        self.openai = openai;
    }

    /// 返回 Anthropic 凭证。对应 Java: `AgentConfig#getAnthropic`。
    #[must_use]
    pub fn anthropic(&self) -> &PlatformCredential {
        &self.anthropic
    }

    /// 返回 Anthropic 头等平台凭证。
    ///
    /// 返回值由 Anthropic ProviderSpec 解析使用。对应 Java:
    /// `AgentConfig#getAnthropic`。
    #[must_use]
    pub fn get_anthropic(&self) -> &PlatformCredential {
        self.anthropic()
    }

    /// 设置 Anthropic 凭证。对应 Java: `AgentConfig#setAnthropic`。
    pub fn set_anthropic(&mut self, anthropic: PlatformCredential) {
        self.anthropic = anthropic;
    }

    /// 返回 Gemini 凭证。对应 Java: `AgentConfig#getGemini`。
    #[must_use]
    pub fn gemini(&self) -> &PlatformCredential {
        &self.gemini
    }

    /// 返回 Gemini 头等平台凭证。
    ///
    /// 返回值由 Gemini ProviderSpec 解析使用。对应 Java:
    /// `AgentConfig#getGemini`。
    #[must_use]
    pub fn get_gemini(&self) -> &PlatformCredential {
        self.gemini()
    }

    /// 设置 Gemini 凭证。对应 Java: `AgentConfig#setGemini`。
    pub fn set_gemini(&mut self, gemini: PlatformCredential) {
        self.gemini = gemini;
    }

    /// 返回 DashScope 凭证。对应 Java: `AgentConfig#getDashscope`。
    #[must_use]
    pub fn dashscope(&self) -> &PlatformCredential {
        &self.dashscope
    }

    /// 返回 DashScope 头等平台凭证。
    ///
    /// 返回值由阿里云百炼 ProviderSpec 解析使用。对应 Java:
    /// `AgentConfig#getDashscope`。
    #[must_use]
    pub fn get_dashscope(&self) -> &PlatformCredential {
        self.dashscope()
    }

    /// 设置 DashScope 凭证。对应 Java: `AgentConfig#setDashscope`。
    pub fn set_dashscope(&mut self, dashscope: PlatformCredential) {
        self.dashscope = dashscope;
    }

    /// 返回 OpenAI 兼容平台凭证。对应 Java: `AgentConfig#getOpenaiCompatible`。
    #[must_use]
    pub fn openai_compatible(&self) -> &HashMap<String, PlatformCredential> {
        &self.openai_compatible
    }

    /// 返回 OpenAI 兼容平台凭证集合。
    ///
    /// Map 的 key 是用户定义的平台名，例如 `deepseek`；返回借用避免复制凭证。
    /// 对应 Java: `AgentConfig#getOpenaiCompatible`。
    #[must_use]
    pub fn get_openai_compatible(&self) -> &HashMap<String, PlatformCredential> {
        self.openai_compatible()
    }

    /// 设置 OpenAI 兼容平台凭证。对应 Java: `AgentConfig#setOpenaiCompatible`。
    pub fn set_openai_compatible(
        &mut self,
        openai_compatible: HashMap<String, PlatformCredential>,
    ) {
        self.openai_compatible = openai_compatible;
    }

    /// 返回 Anthropic 兼容平台凭证。对应 Java: `AgentConfig#getAnthropicCompatible`。
    #[must_use]
    pub fn anthropic_compatible(&self) -> &HashMap<String, PlatformCredential> {
        &self.anthropic_compatible
    }

    /// 返回 Anthropic 兼容平台凭证集合。
    ///
    /// Map 的 key 由带 `compatibleConfigKey` 的 ProviderSpec 查询。对应 Java:
    /// `AgentConfig#getAnthropicCompatible`。
    #[must_use]
    pub fn get_anthropic_compatible(&self) -> &HashMap<String, PlatformCredential> {
        self.anthropic_compatible()
    }

    /// 设置 Anthropic 兼容平台凭证。对应 Java: `AgentConfig#setAnthropicCompatible`。
    pub fn set_anthropic_compatible(
        &mut self,
        anthropic_compatible: HashMap<String, PlatformCredential>,
    ) {
        self.anthropic_compatible = anthropic_compatible;
    }
}
