//! Nacos 规则插件离线契约场景。

use liteflow_core::rule_plugin::{RuleFormat, RuleSource};
use liteflow_rule_nacos::NacosRuleSource;

/// 构建 Nacos 规则源而不访问外部服务。
pub async fn run_case() -> bool {
    let source = NacosRuleSource::new(
        "nacos.example.test:8848",
        "liteflow-flow",
        "DEFAULT_GROUP",
        RuleFormat::Xml,
    );
    source.name() == "nacos" && source.format() == RuleFormat::Xml
}
