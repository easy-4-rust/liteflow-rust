//! 对应 Java: `com.yomahub.liteflow.agent.session.AgentSession`。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::Instant;

use agentscope_core::ReActAgent;
use tokio::sync::Mutex;

use crate::{ChatUsageTrackingHook, ReActLoggingHook, SkillTrackingHook};

/// 单个 Agent 在某次业务会话中的运行时状态。
///
/// `conversation_id` 由整条 chain 共享，`agent_key` 区分同一对话中的不同 Agent；
/// workspace 仅按 conversation 创建，因此多个 Agent 可以通过文件协作。同一
/// `(conversation_id, agent_key)` 的调用由异步 gate 串行化。
///
/// 对应 Java: `com.yomahub.liteflow.agent.session.AgentSession`。
pub struct AgentSession {
    conversation_id: String,
    agent_key: String,
    cache_key: String,
    agent: Arc<ReActAgent>,
    skill_tracking_hook: Option<Arc<SkillTrackingHook>>,
    react_logging_hook: Option<Arc<ReActLoggingHook>>,
    chat_usage_tracking_hook: Arc<ChatUsageTrackingHook>,
    workspace_dir: Option<PathBuf>,
    gate: Mutex<()>,
    last_active: StdMutex<Instant>,
}

impl AgentSession {
    /// 创建缓存项。
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        conversation_id: String,
        agent_key: String,
        cache_key: String,
        agent: Arc<ReActAgent>,
        skill_tracking_hook: Option<Arc<SkillTrackingHook>>,
        react_logging_hook: Option<Arc<ReActLoggingHook>>,
        chat_usage_tracking_hook: Arc<ChatUsageTrackingHook>,
        workspace_dir: Option<PathBuf>,
    ) -> Self {
        Self {
            conversation_id,
            agent_key,
            cache_key,
            agent,
            skill_tracking_hook,
            react_logging_hook,
            chat_usage_tracking_hook,
            workspace_dir,
            gate: Mutex::new(()),
            last_active: StdMutex::new(Instant::now()),
        }
    }

    /// 返回业务对话标识。
    ///
    /// 对应 Java: `AgentSession#getConversationId`。
    #[must_use]
    pub fn conversation_id(&self) -> &str {
        &self.conversation_id
    }

    /// 返回区分同一对话中不同 Agent 的稳定 key。
    ///
    /// 对应 Java: `AgentSession#getAgentKey`。
    #[must_use]
    pub fn agent_key(&self) -> &str {
        &self.agent_key
    }

    /// 返回由 agent key 与 conversation ID 组合得到的缓存 key。
    ///
    /// 对应 Java: `AgentSession#getCacheKey`。
    #[must_use]
    pub fn cache_key(&self) -> &str {
        &self.cache_key
    }

    /// 返回 Agent。
    #[must_use]
    pub fn agent(&self) -> &Arc<ReActAgent> {
        &self.agent
    }

    /// 返回并发闸门。
    ///
    /// 对应 Java: `AgentSession#getLock`。
    #[must_use]
    pub fn gate(&self) -> &Mutex<()> {
        &self.gate
    }

    /// 返回当前对话工作区；未启用工作区能力时返回 `None`。
    ///
    /// 对应 Java: `AgentSession#getWorkspaceDir`。
    #[must_use]
    pub fn workspace_dir(&self) -> Option<&Path> {
        self.workspace_dir.as_deref()
    }

    /// 返回当前会话的 usage 跟踪 Hook。
    ///
    /// 该 Hook 被注入 invocation 级 `ReActAgentContext`，供 `handle_reply` 读取累计值。
    ///
    /// 对应 Java: `AgentSession#getChatUsageTrackingHook`。
    #[must_use]
    pub(crate) fn chat_usage_tracking_hook(&self) -> Arc<ChatUsageTrackingHook> {
        Arc::clone(&self.chat_usage_tracking_hook)
    }

    /// 清空本次 invocation 之前的技能使用记录。
    ///
    /// 对应 Java: `ReActAgentComponent#process` 调用 `SkillTrackingHook#clear`。
    pub(crate) fn clear_used_skills(&self) {
        if let Some(hook) = &self.skill_tracking_hook {
            hook.clear();
        }
    }

    /// 清空本次 invocation 之前的模型 usage 记录。
    ///
    /// 对应 Java: `ReActAgentComponent#process` 调用 `ChatUsageTrackingHook#reset`。
    pub(crate) fn reset_chat_usage(&self) {
        self.chat_usage_tracking_hook.reset();
    }

    /// 返回当前 invocation 的累计 token 和耗时快照。
    ///
    /// 对应 Java: `ReActAgentContext#getChatUsage`。
    #[must_use]
    pub fn chat_usage(&self) -> Option<agentscope_core::message::ChatUsage> {
        self.chat_usage_tracking_hook.snapshot()
    }

    /// 返回当前 invocation 已累计 usage 的 reasoning step 数。
    ///
    /// 对应 Java: `ChatUsageTrackingHook#getSteps`。
    #[must_use]
    pub fn chat_usage_steps(&self) -> usize {
        self.chat_usage_tracking_hook.get_steps()
    }

    /// 返回当前 invocation 已成功加载的技能名称快照。
    ///
    /// 对应 Java: `ReActAgentComponent#usedSkills`。
    #[must_use]
    pub fn used_skills(&self) -> Vec<String> {
        self.skill_tracking_hook
            .as_ref()
            .map_or_else(Vec::new, |hook| hook.get_used_skills())
    }

    /// 记录本次 AgentScope 调用开始。
    pub(crate) fn log_reasoning_start(&self, message_count: usize) {
        if let Some(hook) = &self.react_logging_hook {
            hook.log_reasoning_start(message_count);
        }
    }

    /// 记录本次 AgentScope 调用的最终消息。
    pub(crate) fn log_final_message(&self, message: &agentscope_core::Msg) {
        if let Some(hook) = &self.react_logging_hook {
            hook.log_final_message(message);
        }
    }

    /// 记录本次 AgentScope 调用错误。
    pub(crate) fn log_execution_error(&self, message: &str) {
        if let Some(hook) = &self.react_logging_hook {
            hook.log_execution_error(message);
        }
    }

    /// 刷新最近访问时间。对应 Java: `AgentSession#touch`。
    pub fn touch(&self) {
        *self
            .last_active
            .lock()
            .expect("agent session last_active lock poisoned") = Instant::now();
    }

    /// 返回最近访问时间。对应 Java: `AgentSession#getLastActive`。
    pub fn last_active(&self) -> Instant {
        *self
            .last_active
            .lock()
            .expect("agent session last_active lock poisoned")
    }
}
