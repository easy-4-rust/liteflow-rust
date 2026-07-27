//! Etcd 规则插件离线契约场景。

use liteflow_core::rule_plugin::{RuleFormat, RuleSource};
use liteflow_rule_etcd::EtcdRuleSource;

/// 构建 Etcd 规则源而不访问外部服务。
pub async fn run_case() -> bool {
    let source = EtcdRuleSource::new(
        vec!["http://etcd.example.test:2379".to_string()],
        "/liteflow/flow",
    )
    .expect("Etcd 离线契约配置应有效");
    source.name() == "etcd" && source.format() == RuleFormat::Xml
}
