use serde::{Deserialize, Serialize};

/// AgentScope Skills 配置，对应配置段 `liteflow.agent.skills.*`。
///
/// 对应 Java: `com.yomahub.liteflow.property.agent.SkillsConfig`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SkillsConfig {
    /// 是否启用配置驱动的 Skills，默认关闭。
    pub enabled: bool,
    /// Skills 根目录，默认当前工作目录下的 `./skills`。
    pub path: String,
    /// 是否严格解析非法配置，默认开启。
    pub strict: bool,
}

impl Default for SkillsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            path: "./skills".to_string(),
            strict: true,
        }
    }
}

impl SkillsConfig {
    /// 返回是否启用 Skills。对应 Java: `SkillsConfig#isEnabled`。
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 设置 Skills 开关。对应 Java: `SkillsConfig#setEnabled`。
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// 返回 Skills 根目录。对应 Java: `SkillsConfig#getPath`。
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// 返回 Skills 根目录。
    ///
    /// - 返回：技能加载器实际使用的目录；默认值为 `./skills`。
    ///
    /// 对应 Java: `SkillsConfig#getPath`。
    #[must_use]
    pub fn get_path(&self) -> &str {
        self.path()
    }

    /// 设置 Skills 根目录。对应 Java: `SkillsConfig#setPath`。
    pub fn set_path(&mut self, path: impl Into<String>) {
        self.path = path.into();
    }

    /// 返回是否严格解析。对应 Java: `SkillsConfig#isStrict`。
    #[must_use]
    pub fn is_strict(&self) -> bool {
        self.strict
    }

    /// 设置严格解析开关。对应 Java: `SkillsConfig#setStrict`。
    pub fn set_strict(&mut self, strict: bool) {
        self.strict = strict;
    }
}
