use std::collections::HashMap;
use std::sync::Arc;

use agentscope_core::tool::agent_tool::PermissionContextState;
use agentscope_core::tool::{AgentTool, PermissionDecision, ToolContext, ToolResult};
use async_trait::async_trait;

/// 为已配置的 AgentScope 技能加载工具提供精确权限适配。
///
/// 该对象保留 AgentScope 原工具的 Schema、执行逻辑和并发属性，只将
/// `load_skill_through_path` 的权限自检改为允许。适配范围不会覆盖 Shell、
/// 工作区写入或用户注册的其他工具。
///
/// 对应 Java: `SkillBoxFactory#build` 创建的技能加载工具可直接执行语义。
pub(crate) struct SkillLoadPermissionTool {
    delegate: Arc<dyn AgentTool>,
}

impl SkillLoadPermissionTool {
    /// 包装 AgentScope SkillBox 已注册的真实技能加载工具。
    ///
    /// # 参数
    /// - `delegate`: SkillBox 内部注册到会话 Toolkit 的原始工具。
    ///
    /// # 返回
    /// 仅覆盖权限决策、其余行为全部委托的工具适配器。
    #[must_use]
    pub(crate) fn new(delegate: Arc<dyn AgentTool>) -> Self {
        Self { delegate }
    }
}

#[async_trait]
impl AgentTool for SkillLoadPermissionTool {
    fn name(&self) -> &str {
        self.delegate.name()
    }

    fn description(&self) -> &str {
        self.delegate.description()
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.delegate.parameters_schema()
    }

    fn strict(&self) -> Option<bool> {
        self.delegate.strict()
    }

    fn output_schema(&self) -> Option<serde_json::Value> {
        self.delegate.output_schema()
    }

    fn is_read_only(&self) -> bool {
        self.delegate.is_read_only()
    }

    fn is_concurrency_safe(&self) -> bool {
        self.delegate.is_concurrency_safe()
    }

    async fn execute(&self, context: ToolContext) -> ToolResult {
        self.delegate.execute(context).await
    }

    fn check_permissions(
        &self,
        _tool_input: &HashMap<String, serde_json::Value>,
        _context: &PermissionContextState,
    ) -> PermissionDecision {
        PermissionDecision::allow("LiteFlow configured local skill loader")
    }
}
