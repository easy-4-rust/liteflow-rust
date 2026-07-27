use std::collections::HashMap;
use std::sync::Arc;

use agentscope_core::skill::AgentSkill;
use agentscope_core::tool::AgentTool;

use crate::{AgentConfigException, SkillsConfig};

/// Rust 工具工厂的全局注册描述。
///
/// Rust 不支持 Java 的 `Class.forName` 与无参构造器反射，因此业务工具通过
/// `inventory::submit!` 显式注册稳定名称和真实构造函数。名称应与 `SKILL.md`
/// frontmatter 的 `tools` 条目完全一致。
///
/// 对应 Java: `SkillToolResolver#resolveToolInstance` 的 Rust SPI 映射。
pub struct SkillToolRegistration {
    /// frontmatter 中引用的稳定工具类型名称。
    pub type_name: &'static str,
    /// 创建真实 AgentScope 工具实例的构造函数。
    pub factory: fn() -> Result<Arc<dyn AgentTool>, AgentConfigException>,
}

inventory::collect!(SkillToolRegistration);

/// 把技能 frontmatter 中声明的 `tools` 解析为可注册的 AgentScope 工具实例。
///
/// 解析只作用于传入的已选技能，不会读取或校验其它技能。严格模式下，未知工具名
/// 或构造失败会返回配置错误；宽松模式下记录警告并继续处理其余工具。
///
/// 对应 Java: `com.yomahub.liteflow.agent.skill.SkillToolResolver`。
pub(crate) struct SkillToolResolver {
    config: SkillsConfig,
}

impl SkillToolResolver {
    const TOOLS_METADATA_KEY: &'static str = "tools";

    /// 使用 Skills 配置创建工具解析器。
    pub(crate) fn new(config: &SkillsConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// 解析并实例化指定技能声明的全部工具。
    ///
    /// 对应 Java: `SkillToolResolver#instantiateTools`。
    pub(crate) fn instantiate_tools(
        &self,
        skill: &AgentSkill,
    ) -> Result<Vec<Arc<dyn AgentTool>>, AgentConfigException> {
        let registrations = inventory::iter::<SkillToolRegistration>
            .into_iter()
            .map(|registration| (registration.type_name, registration))
            .collect::<HashMap<_, _>>();
        let mut instances = Vec::new();

        for type_name in self.tool_type_names(skill) {
            let Some(registration) = registrations.get(type_name.as_str()) else {
                self.handle_problem(format!(
                    "Skill '{}' references unknown tool class '{}'",
                    skill.name, type_name
                ))?;
                continue;
            };
            match (registration.factory)() {
                Ok(tool) => instances.push(tool),
                Err(error) => {
                    self.handle_problem(format!(
                        "Skill '{}' tool class '{}' instantiation failed: {}",
                        skill.name, type_name, error
                    ))?;
                }
            }
        }
        Ok(instances)
    }

    fn tool_type_names(&self, skill: &AgentSkill) -> Vec<String> {
        if !skill.tool_names.is_empty() {
            return skill
                .tool_names
                .iter()
                .map(|name| name.trim())
                .filter(|name| !name.is_empty())
                .map(ToOwned::to_owned)
                .collect();
        }

        let Some(field) = skill.get_metadata_value(Self::TOOLS_METADATA_KEY) else {
            return Vec::new();
        };
        match field {
            serde_json::Value::Array(values) => values
                .iter()
                .map(value_text)
                .map(|name| name.trim().to_string())
                .filter(|name| !name.is_empty())
                .collect(),
            value => value_text(value)
                .split(',')
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
        }
    }

    fn handle_problem(&self, message: String) -> Result<(), AgentConfigException> {
        if self.config.strict {
            return Err(AgentConfigException::new(message));
        }
        tracing::warn!("{message}");
        Ok(())
    }
}

fn value_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => value.clone(),
        value => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use agentscope_core::tool::{AgentTool, ToolContext, ToolResult};
    use async_trait::async_trait;
    use serde_json::json;

    use super::{SkillToolRegistration, SkillToolResolver};
    use crate::{AgentConfigException, SkillsConfig};

    struct ResolverTestTool;

    #[async_trait]
    impl AgentTool for ResolverTestTool {
        fn name(&self) -> &str {
            "resolver_test_tool"
        }

        fn description(&self) -> &str {
            "验证 SkillToolResolver 注册解析"
        }

        fn parameters_schema(&self) -> serde_json::Value {
            json!({"type": "object", "properties": {}})
        }

        async fn execute(&self, _context: ToolContext) -> ToolResult {
            ToolResult::success("ok")
        }
    }

    inventory::submit! {
        SkillToolRegistration {
            type_name: "tests.ResolverTestTool",
            factory: || Ok(Arc::new(ResolverTestTool)),
        }
    }

    inventory::submit! {
        SkillToolRegistration {
            type_name: "tests.FailingTool",
            factory: || Err(AgentConfigException::new("factory failed")),
        }
    }

    fn skill_with_tools(tools: serde_json::Value) -> agentscope_core::skill::AgentSkill {
        let mut metadata = HashMap::new();
        metadata.insert("name".to_string(), json!("resolver-test"));
        metadata.insert("description".to_string(), json!("resolver test skill"));
        metadata.insert("tools".to_string(), tools);
        agentscope_core::skill::AgentSkill::from_metadata(
            metadata,
            "Use the registered tool.",
            HashMap::new(),
            "test",
        )
        .expect("skill metadata should be valid")
    }

    #[test]
    fn resolves_registered_tool_from_inline_array_metadata() {
        let resolver = SkillToolResolver::new(&SkillsConfig::default());
        let tools = resolver
            .instantiate_tools(&skill_with_tools(json!(["tests.ResolverTestTool"])))
            .expect("registered tool should resolve");

        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name(), "resolver_test_tool");
    }

    #[test]
    fn strict_and_lenient_unknown_tool_policies_match_java() {
        let strict = SkillToolResolver::new(&SkillsConfig::default());
        let error = match strict.instantiate_tools(&skill_with_tools(json!("missing.Tool"))) {
            Ok(_) => panic!("strict mode should reject an unknown tool"),
            Err(error) => error,
        };
        assert!(error.message().contains("missing.Tool"));

        let config = SkillsConfig {
            strict: false,
            ..SkillsConfig::default()
        };
        let lenient = SkillToolResolver::new(&config);
        assert!(
            lenient
                .instantiate_tools(&skill_with_tools(json!("missing.Tool")))
                .expect("lenient mode should skip the unknown tool")
                .is_empty()
        );
    }

    #[test]
    fn tool_factory_failure_is_reported_as_configuration_error() {
        let resolver = SkillToolResolver::new(&SkillsConfig::default());
        let error = match resolver.instantiate_tools(&skill_with_tools(json!("tests.FailingTool")))
        {
            Ok(_) => panic!("strict mode should surface a factory failure"),
            Err(error) => error,
        };

        assert!(error.message().contains("factory failed"));
    }
}
