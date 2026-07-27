use std::collections::HashMap;
use std::sync::Arc;

use agentscope_core::skill::SkillBox;
use agentscope_core::tool::AgentTool;

/// Skills 加载结果，保存技能箱、技能 ID 到名称的映射及实际加载名称。
///
/// 对应 Java: `com.yomahub.liteflow.agent.skill.SkillLoadResult`。
pub struct SkillLoadResult {
    skill_box: Arc<SkillBox>,
    skill_id_to_name: HashMap<String, String>,
    skill_names: Vec<String>,
    skill_tools: HashMap<String, Vec<Arc<dyn AgentTool>>>,
}

impl SkillLoadResult {
    /// 创建不可变 Skills 加载结果。
    ///
    /// # 参数
    /// - `skill_box`: 已注册所选技能的 AgentScope 技能箱。
    /// - `skill_id_to_name`: 技能目录 ID 到展示名称的映射。
    /// - `skill_names`: 按选择顺序保存的技能名称。
    /// - `skill_tools`: 技能 ID 到该技能声明工具实例的映射。
    ///
    /// # 返回
    /// 可供 Agent 构建和跟踪 Hook 消费的加载结果。
    ///
    /// 对应 Java: `SkillLoadResult#SkillLoadResult`。
    #[must_use]
    pub fn new(
        skill_box: Arc<SkillBox>,
        skill_id_to_name: HashMap<String, String>,
        skill_names: Vec<String>,
        skill_tools: HashMap<String, Vec<Arc<dyn AgentTool>>>,
    ) -> Self {
        Self {
            skill_box,
            skill_id_to_name,
            skill_names,
            skill_tools,
        }
    }

    /// 返回共享的 AgentScope 技能箱。对应 Java: `SkillLoadResult#skillBox`。
    #[must_use]
    pub fn skill_box(&self) -> &Arc<SkillBox> {
        &self.skill_box
    }

    /// 返回技能 ID 到名称的只读映射。对应 Java: `SkillLoadResult#skillIdToName`。
    #[must_use]
    pub fn skill_id_to_name(&self) -> &HashMap<String, String> {
        &self.skill_id_to_name
    }

    /// 返回实际加载的技能名称。对应 Java: `SkillLoadResult#skillNames`。
    #[must_use]
    pub fn skill_names(&self) -> &[String] {
        &self.skill_names
    }

    /// 返回待绑定到对应技能工具组的真实工具实例。
    ///
    /// 工具只在 `SkillBox` 绑定 Agent 本地 Toolkit 后注册，从而避免注册到构建前
    /// 被深拷贝丢弃的临时 Toolkit。
    ///
    /// 对应 Java: `SkillBoxFactory#build` 中的逐技能工具注册结果。
    pub(crate) fn skill_tools(&self) -> &HashMap<String, Vec<Arc<dyn AgentTool>>> {
        &self.skill_tools
    }
}
