//! ZooKeeper 规则插件离线契约场景。

use liteflow_core::rule_plugin::{RuleFormat, RuleSource};
use liteflow_rule_zk::ZkRuleSource;

/// 构建 ZooKeeper 规则源而不访问外部服务。
pub async fn run_case() -> bool {
    let source = ZkRuleSource {
        connect_string: "zk.example.test:2181".to_string(),
        node_path: "/liteflow/flow".to_string(),
        format: RuleFormat::Xml,
    };
    source.name() == "zookeeper" && source.format() == RuleFormat::Xml
}
