use std::path::{Path, PathBuf};
use std::sync::Arc;

use agentscope_core::message::ChatUsage;
use liteflow_core::CmpContext;

use crate::ChatUsageTrackingHook;

/// 在 `ReActAgentComponent` 扩展点中暴露的单次调用运行时上下文。
///
/// 上下文包含三个相互独立的标识层：
///
/// - `conversation_id`：业务对话维度，在同一条 chain 内保持一致；
/// - `agent_key`：Agent 组件维度，默认等于节点 ID；
/// - `workspace_dir`：对话工作区，同一对话中的多个 Agent 可以共享。
///
/// 本对象只在一次 `process` 调用期间有效。不得由跨调用缓存的工具、Hook 或 Model
/// 长期持有；需要访问时应通过 `ReActAgentComponent::runtime_context` 动态获取当次对象。
///
/// 对应 Java: `com.yomahub.liteflow.agent.component.ReActAgentContext`。
#[derive(Clone)]
pub struct ReActAgentContext {
    cmp_context: CmpContext,
    conversation_id: String,
    agent_key: String,
    workspace_dir: Option<PathBuf>,
    chat_usage_tracking_hook: Arc<ChatUsageTrackingHook>,
}

impl ReActAgentContext {
    /// 创建单次调用上下文并注入当前会话的 usage 跟踪 Hook。
    ///
    /// Rust 在构造时完成 Java `setChatUsageTrackingHook` 的框架注入，避免扩展点观察到
    /// 尚未初始化的中间状态。
    ///
    /// 对应 Java: `ReActAgentContext#ReActAgentContext`、
    /// `ReActAgentContext#setChatUsageTrackingHook`。
    pub(crate) fn new(
        cmp_context: CmpContext,
        conversation_id: impl Into<String>,
        agent_key: impl Into<String>,
        workspace_dir: Option<PathBuf>,
        chat_usage_tracking_hook: Arc<ChatUsageTrackingHook>,
    ) -> Self {
        Self {
            cmp_context,
            conversation_id: conversation_id.into(),
            agent_key: agent_key.into(),
            workspace_dir,
            chat_usage_tracking_hook,
        }
    }

    /// 返回当前 LiteFlow 节点执行上下文。
    ///
    /// Rust 的 `CmpContext` 对应 Java `Slot` 在当前节点上的类型安全访问视图。
    ///
    /// 对应 Java: `ReActAgentContext#getSlot`。
    #[must_use]
    pub fn cmp_context(&self) -> &CmpContext {
        &self.cmp_context
    }

    /// 返回业务对话标识。
    ///
    /// 对应 Java: `ReActAgentContext#getConversationId`。
    #[must_use]
    pub fn conversation_id(&self) -> &str {
        &self.conversation_id
    }

    /// 返回用于区分同一对话中不同 Agent 的稳定 key。
    ///
    /// 对应 Java: `ReActAgentContext#getAgentKey`。
    #[must_use]
    pub fn agent_key(&self) -> &str {
        &self.agent_key
    }

    /// 返回当前对话工作区；未启用工作区能力时返回 `None`。
    ///
    /// Java 实现始终创建工作区；Rust 仅在文件、Shell 或 Skills 能力需要时创建，
    /// 因而使用 `Option` 明确表达未配置状态。
    ///
    /// 对应 Java: `ReActAgentContext#getWorkspaceDir`。
    #[must_use]
    pub fn workspace_dir(&self) -> Option<&Path> {
        self.workspace_dir.as_deref()
    }

    /// 返回本次 `process` 截至当前累计的 token 用量与推理耗时。
    ///
    /// 在 `handle_reply` 回调中调用时，返回整次 Agent 调用的累计值；若模型没有上报
    /// usage，或尚未观察到有效 usage，则返回 `None`。
    ///
    /// 对应 Java: `ReActAgentContext#getChatUsage`。
    #[must_use]
    pub fn chat_usage(&self) -> Option<ChatUsage> {
        self.chat_usage_tracking_hook.snapshot()
    }
}
