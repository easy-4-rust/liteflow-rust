use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use agentscope_core::skill::filesystem_repository::FileSystemSkillRepository;
use agentscope_core::skill::{AgentSkill, AgentSkillRepository, SkillBox};

use super::SkillToolResolver;
use crate::{AgentConfig, AgentError, SkillLoadResult};

/// 根据 LiteFlow 配置加载文件系统 Skills 并构造 AgentScope SkillBox。
///
/// Rust 版沿用 Java 的严格/宽松失败策略和按名称筛选规则。AgentScope-Rust 的
/// `ReActAgentBuilder#skill_box` 会在构建阶段绑定其内部 Toolkit，因此工厂不接收
/// Java 版的 Toolkit 参数。
///
/// 对应 Java: `com.yomahub.liteflow.agent.skill.SkillBoxFactory`。
pub struct SkillBoxFactory;

impl SkillBoxFactory {
    /// 从配置目录加载并筛选 Skills。
    ///
    /// # 参数
    /// - `agent_config`: LiteFlow Agent 根配置。
    /// - `allowed_skills`: 允许使用的技能名称；空集合表示加载全部技能。
    /// - `workspace_dir`: 可选会话工作区，用于 AgentScope 技能代码执行目录。
    ///
    /// # 返回
    /// 已构建的技能箱、技能映射和实际加载名称。
    ///
    /// # 错误
    /// 严格模式下，目录缺失、仓库加载失败或声明技能不存在会返回 `AgentError`。
    ///
    /// 对应 Java: `SkillBoxFactory#build`。
    pub async fn build(
        agent_config: &AgentConfig,
        allowed_skills: &[String],
        workspace_dir: Option<&Path>,
    ) -> Result<SkillLoadResult, AgentError> {
        let skills_config = &agent_config.skills;
        let root = PathBuf::from(&skills_config.path);
        if !root.is_dir() {
            return Self::handle_missing_root(&root, skills_config.strict, workspace_dir);
        }

        let repository = match FileSystemSkillRepository::read_only(&root) {
            Ok(repository) => repository,
            Err(error) => {
                return Self::handle_problem(
                    format!("Failed to load skills from {}: {error}", root.display()),
                    skills_config.strict,
                    workspace_dir,
                );
            }
        };
        let all_skills = match repository.list_skills().await {
            Ok(skills) => skills,
            Err(error) => {
                return Self::handle_problem(
                    format!("Failed to load skills from {}: {error}", root.display()),
                    skills_config.strict,
                    workspace_dir,
                );
            }
        };
        let selected = Self::select_skills(all_skills, allowed_skills, skills_config.strict)?;
        let skill_box = Self::create_skill_box(workspace_dir);
        let mut skill_id_to_name = HashMap::new();
        let mut skill_names = Vec::with_capacity(selected.len());
        let mut skill_tools = HashMap::new();
        let tool_resolver = SkillToolResolver::new(skills_config);

        // 先解析所选技能声明的工具；真正注册要等 SkillBox 绑定 Agent 本地 Toolkit。
        for skill in selected {
            skill_id_to_name.insert(skill.skill_id.clone(), skill.name.clone());
            skill_names.push(skill.name.clone());
            let tools = tool_resolver
                .instantiate_tools(&skill)
                .map_err(AgentError::from)?;
            if !tools.is_empty() {
                skill_tools.insert(skill.skill_id.clone(), tools);
            }
            skill_box.register(skill);
        }

        Ok(SkillLoadResult::new(
            skill_box,
            skill_id_to_name,
            skill_names,
            skill_tools,
        ))
    }

    fn handle_missing_root(
        root: &Path,
        strict: bool,
        workspace_dir: Option<&Path>,
    ) -> Result<SkillLoadResult, AgentError> {
        Self::handle_problem(
            format!("Skills root not found: {}", root.display()),
            strict,
            workspace_dir,
        )
    }

    fn handle_problem(
        message: String,
        strict: bool,
        workspace_dir: Option<&Path>,
    ) -> Result<SkillLoadResult, AgentError> {
        if strict {
            return Err(AgentError::SkillsLoad(message));
        }
        tracing::warn!("{message}");
        Ok(SkillLoadResult::new(
            Self::create_skill_box(workspace_dir),
            HashMap::new(),
            Vec::new(),
            HashMap::new(),
        ))
    }

    fn create_skill_box(workspace_dir: Option<&Path>) -> Arc<SkillBox> {
        let mut skill_box = SkillBox::new(None);
        if let Some(workspace_dir) = workspace_dir {
            skill_box.set_work_dir(workspace_dir.to_path_buf());
            skill_box.set_code_execution_enabled(true);
        }
        Arc::new(skill_box)
    }

    fn select_skills(
        all_skills: Vec<AgentSkill>,
        allowed_skills: &[String],
        strict: bool,
    ) -> Result<Vec<AgentSkill>, AgentError> {
        let mut by_name = all_skills
            .into_iter()
            .map(|skill| (skill.name.clone(), skill))
            .collect::<HashMap<_, _>>();
        let allowed = Self::normalize_allowed_skills(allowed_skills);
        if allowed.is_empty() {
            let mut selected = by_name.into_values().collect::<Vec<_>>();
            selected.sort_by(|left, right| left.name.cmp(&right.name));
            return Ok(selected);
        }

        let missing = allowed
            .iter()
            .filter(|name| !by_name.contains_key(name.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            let message = format!("Declared skills not found: {missing:?}");
            if strict {
                return Err(AgentError::SkillsLoad(message));
            }
            tracing::warn!("{message}");
        }

        Ok(allowed
            .into_iter()
            .filter_map(|name| by_name.remove(&name))
            .collect())
    }

    fn normalize_allowed_skills(allowed_skills: &[String]) -> Vec<String> {
        let mut seen = HashSet::new();
        allowed_skills
            .iter()
            .map(|name| name.trim())
            .filter(|name| !name.is_empty())
            .filter(|name| seen.insert((*name).to_string()))
            .map(ToOwned::to_owned)
            .collect()
    }
}
