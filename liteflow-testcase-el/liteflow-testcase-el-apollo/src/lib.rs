//! Apollo 规则插件离线契约场景。

use liteflow_core::rule_plugin::{RuleFormat, RuleSource};
use liteflow_rule_apollo::ApolloRuleSource;

/// 构建 Apollo 规则源而不访问外部服务。
pub async fn run_case() -> bool {
    let source = ApolloRuleSource {
        portal_addr: "apollo.example.test".to_string(),
        app_id: "liteflow".to_string(),
        cluster: "default".to_string(),
        namespace: "application".to_string(),
        key: "flow".to_string(),
        format: RuleFormat::Xml,
    };
    source.name() == "apollo" && source.format() == RuleFormat::Xml
}
