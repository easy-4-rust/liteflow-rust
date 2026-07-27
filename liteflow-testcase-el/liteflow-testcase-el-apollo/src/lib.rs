//! Apollo 规则插件离线契约场景。

use liteflow_core::rule_plugin::{RuleFormat, RuleSource};
use liteflow_rule_apollo::ApolloRuleSource;

/// 构建 Apollo 规则源而不访问外部服务。
pub async fn run_case() -> bool {
    let source = ApolloRuleSource::new("apollo.example.test", "liteflow", "default", "application")
        .expect("Apollo 离线契约配置应有效");
    source.name() == "apollo" && source.format() == RuleFormat::Xml
}
