//! LiteFlow Skills 配置、筛选和失败策略的真实文件系统测试。

use std::fs;
use std::path::Path;

use liteflow_agent_core::{AgentConfig, AgentError, SkillBoxFactory};

fn write_skill(root: &Path, directory_name: &str, skill_name: &str) {
    let skill_directory = root.join(directory_name);
    fs::create_dir_all(&skill_directory).expect("应创建技能目录");
    fs::write(
        skill_directory.join("SKILL.md"),
        format!(
            "---\nname: {skill_name}\ndescription: {skill_name} 测试技能\n---\n\n# {skill_name}\n\n请执行测试任务。"
        ),
    )
    .expect("应写入真实 SKILL.md");
}

fn skills_config(root: &Path, strict: bool) -> AgentConfig {
    let mut config = AgentConfig::default();
    config.skills.enabled = true;
    config.skills.path = root.to_string_lossy().into_owned();
    config.skills.strict = strict;
    config
}

#[tokio::test]
async fn empty_allow_list_loads_all_skills_in_name_order() {
    let directory = tempfile::tempdir().expect("应创建临时目录");
    write_skill(directory.path(), "zulu", "Zulu");
    write_skill(directory.path(), "alpha", "Alpha");

    let result = SkillBoxFactory::build(
        &skills_config(directory.path(), true),
        &[],
        Some(directory.path()),
    )
    .await
    .expect("空允许列表应加载全部技能");

    assert_eq!(result.skill_names(), &["Alpha", "Zulu"]);
    assert_eq!(
        result.skill_id_to_name().get("alpha"),
        Some(&"Alpha".to_string())
    );
    assert_eq!(
        result.skill_id_to_name().get("zulu"),
        Some(&"Zulu".to_string())
    );
}

#[tokio::test]
async fn declared_skills_preserve_order_and_remove_duplicates() {
    let directory = tempfile::tempdir().expect("应创建临时目录");
    write_skill(directory.path(), "alpha", "Alpha");
    write_skill(directory.path(), "beta", "Beta");

    let allowed = vec![
        " Beta ".to_string(),
        "Alpha".to_string(),
        "Beta".to_string(),
    ];
    let result = SkillBoxFactory::build(&skills_config(directory.path(), true), &allowed, None)
        .await
        .expect("声明技能应按声明顺序筛选");

    assert_eq!(result.skill_names(), &["Beta", "Alpha"]);
}

#[tokio::test]
async fn strict_mode_rejects_missing_root_and_declared_skill() {
    let directory = tempfile::tempdir().expect("应创建临时目录");
    let missing_root = directory.path().join("missing");
    let root_error =
        match SkillBoxFactory::build(&skills_config(&missing_root, true), &[], None).await {
            Err(error) => error,
            Ok(_) => panic!("严格模式应拒绝缺失目录"),
        };
    assert!(matches!(root_error, AgentError::SkillsLoad(_)));

    write_skill(directory.path(), "alpha", "Alpha");
    let missing_skill = match SkillBoxFactory::build(
        &skills_config(directory.path(), true),
        &["Unknown".to_string()],
        None,
    )
    .await
    {
        Err(error) => error,
        Ok(_) => panic!("严格模式应拒绝未找到的声明技能"),
    };
    assert!(matches!(missing_skill, AgentError::SkillsLoad(_)));
}

#[tokio::test]
async fn non_strict_mode_returns_available_or_empty_skill_box() {
    let directory = tempfile::tempdir().expect("应创建临时目录");
    let missing_root = directory.path().join("missing");
    let empty = SkillBoxFactory::build(
        &skills_config(&missing_root, false),
        &["Unknown".to_string()],
        None,
    )
    .await
    .expect("宽松模式应容忍缺失目录");
    assert!(empty.skill_names().is_empty());

    write_skill(directory.path(), "alpha", "Alpha");
    let selected = SkillBoxFactory::build(
        &skills_config(directory.path(), false),
        &["Unknown".to_string(), "Alpha".to_string()],
        None,
    )
    .await
    .expect("宽松模式应跳过不存在的声明技能");
    assert_eq!(selected.skill_names(), &["Alpha"]);
}
