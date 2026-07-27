//! Java `SkillsConfig` 默认值和目录访问器语义回归测试。

use liteflow_core::property::agent::SkillsConfig;

/// 验证技能目录默认值及 Java 命名 getter 与 Agent 消费入口共享同一字段。
#[test]
fn path_getter_preserves_java_default_and_mutation() {
    let mut config = SkillsConfig::default();
    assert_eq!(config.get_path(), "./skills");

    config.set_path("/srv/liteflow/skills");
    assert_eq!(config.get_path(), "/srv/liteflow/skills");
    assert_eq!(config.path(), "/srv/liteflow/skills");
}
