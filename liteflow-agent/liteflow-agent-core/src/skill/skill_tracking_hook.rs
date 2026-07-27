use std::collections::HashMap;
use std::sync::Mutex;

use agentscope_core::ContentBlock;
use agentscope_core::agent::AgentError as AgentScopeError;
use agentscope_core::hook::{Hook, HookEvent};
use agentscope_core::message::{ToolResultBlock, ToolUseBlock};
use async_trait::async_trait;

/// 跟踪 ReAct 会话中通过 AgentScope 加载工具成功加载的技能。
///
/// 使用互斥保护的有序列表保存首次使用顺序，同一技能只记录一次。
///
/// 对应 Java: `com.yomahub.liteflow.agent.skill.SkillTrackingHook`。
pub struct SkillTrackingHook {
    skill_id_to_name: HashMap<String, String>,
    used_skills: Mutex<Vec<String>>,
}

impl SkillTrackingHook {
    /// AgentScope 内置技能加载工具名称。
    pub const LOAD_SKILL_TOOL_NAME: &'static str = "load_skill_through_path";
    const SKILL_ID_INPUT_KEY: &'static str = "skillId";

    /// 创建技能使用跟踪 Hook。
    ///
    /// # 参数
    /// - `skill_id_to_name`: 技能目录 ID 到展示名称的映射。
    ///
    /// # 返回
    /// 可注册到 ReActAgent 的线程安全 Hook。
    ///
    /// 对应 Java: `SkillTrackingHook#SkillTrackingHook`。
    #[must_use]
    pub fn new(skill_id_to_name: HashMap<String, String>) -> Self {
        Self {
            skill_id_to_name,
            used_skills: Mutex::new(Vec::new()),
        }
    }

    /// 返回按首次加载顺序去重的技能名称快照。
    ///
    /// 对应 Java: `SkillTrackingHook#getUsedSkills`。
    #[must_use]
    pub fn get_used_skills(&self) -> Vec<String> {
        self.used_skills
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// 清空本次 invocation 的技能使用记录。
    ///
    /// 对应 Java: `SkillTrackingHook#clear`。
    pub fn clear(&self) {
        self.used_skills
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }

    fn record_skill_load(&self, tool_use: &ToolUseBlock, tool_result: Option<&ToolResultBlock>) {
        if tool_use.name != Self::LOAD_SKILL_TOOL_NAME
            || tool_result.is_some_and(Self::is_error_result)
        {
            return;
        }
        let Some(skill_id) = tool_use.input.get(Self::SKILL_ID_INPUT_KEY) else {
            return;
        };
        let skill_id = skill_id
            .as_str()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| skill_id.to_string());
        let Some(skill_name) = self.skill_id_to_name.get(&skill_id) else {
            return;
        };
        let mut used_skills = self
            .used_skills
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !used_skills.contains(skill_name) {
            used_skills.push(skill_name.clone());
        }
    }

    fn is_error_result(tool_result: &ToolResultBlock) -> bool {
        tool_result.output.iter().any(
            |block| matches!(block, ContentBlock::Text(text) if text.text.starts_with("Error:")),
        )
    }
}

#[async_trait]
impl Hook for SkillTrackingHook {
    async fn on_event(&self, event: HookEvent) -> Result<HookEvent, AgentScopeError> {
        if let HookEvent::PostActing(data) = &event {
            self.record_skill_load(&data.tool_use, data.tool_result.as_ref());
        }
        Ok(event)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use agentscope_core::hook::{Hook, HookEvent, PostActingEvent};
    use agentscope_core::message::{ToolResultBlock, ToolUseBlock};
    use serde_json::json;

    use super::SkillTrackingHook;

    fn event(result_text: &str) -> HookEvent {
        HookEvent::PostActing(PostActingEvent {
            tool_use: ToolUseBlock::new(
                "call-1",
                SkillTrackingHook::LOAD_SKILL_TOOL_NAME,
                HashMap::from([("skillId".to_string(), json!("beta"))]),
            ),
            tool_result: Some(ToolResultBlock::text(result_text)),
            tool_result_msg: None,
            stop_requested: false,
            toolkit_name: "skills".to_string(),
        })
    }

    #[tokio::test]
    async fn records_success_once_and_ignores_error_result() {
        let hook =
            SkillTrackingHook::new(HashMap::from([("beta".to_string(), "Beta".to_string())]));

        hook.on_event(event("loaded"))
            .await
            .expect("成功事件应透传");
        hook.on_event(event("loaded"))
            .await
            .expect("重复事件应透传");
        assert_eq!(hook.get_used_skills(), vec!["Beta"]);

        hook.clear();
        hook.on_event(event("Error: missing"))
            .await
            .expect("失败事件应透传");
        assert!(hook.get_used_skills().is_empty());
    }
}
