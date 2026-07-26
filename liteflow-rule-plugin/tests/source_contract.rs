use liteflow_core::rule_plugin::{RuleFormat, RuleSource};

#[cfg(feature = "apollo")]
#[test]
fn apollo_source_exposes_rule_contract_without_connecting() {
    let source = liteflow_rule_plugin::apollo::ApolloRuleSource {
        portal_addr: "127.0.0.1:8070".to_string(),
        app_id: "sample".to_string(),
        cluster: "default".to_string(),
        namespace: "application".to_string(),
        key: "flow".to_string(),
        format: RuleFormat::Xml,
    };
    assert_eq!(source.name(), "apollo");
    assert_eq!(source.format(), RuleFormat::Xml);
}

#[cfg(feature = "etcd")]
#[test]
fn etcd_source_constructor_preserves_contract_metadata() {
    let source = liteflow_rule_plugin::etcd::EtcdRuleSource::new(
        vec!["http://127.0.0.1:2379".to_string()],
        "/liteflow/flow",
        RuleFormat::Json,
    )
    .with_auth("user", "password");
    assert_eq!(source.name(), "etcd");
    assert_eq!(source.format(), RuleFormat::Json);
}

#[cfg(feature = "nacos")]
#[test]
fn nacos_source_constructor_preserves_contract_metadata() {
    let source = liteflow_rule_plugin::nacos::NacosRuleSource::new(
        "127.0.0.1:8848",
        "flow.xml",
        "DEFAULT_GROUP",
        RuleFormat::Xml,
    )
    .with_namespace("tenant")
    .with_auth("nacos", "nacos");
    assert_eq!(source.name(), "nacos");
    assert_eq!(source.format(), RuleFormat::Xml);
}

#[cfg(feature = "redis")]
#[test]
fn redis_source_exposes_rule_contract_without_connecting() {
    let source = liteflow_rule_plugin::redis::RedisRuleSource {
        url: "redis://127.0.0.1/".to_string(),
        key: "liteflow:flow".to_string(),
        format: RuleFormat::Yml,
    };
    assert_eq!(source.name(), "redis");
    assert_eq!(source.format(), RuleFormat::Yml);
}

#[cfg(feature = "zk")]
#[test]
fn zk_source_exposes_rule_contract_without_connecting() {
    let source = liteflow_rule_plugin::zk::ZkRuleSource {
        connect_string: "127.0.0.1:2181".to_string(),
        node_path: "/lite-flow/flow".to_string(),
        format: RuleFormat::Json,
    };
    assert_eq!(source.name(), "zookeeper");
    assert_eq!(source.format(), RuleFormat::Json);
}
