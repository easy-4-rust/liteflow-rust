#[cfg(any(
    feature = "apollo",
    feature = "etcd",
    feature = "nacos",
    feature = "redis"
))]
use liteflow_core::rule_plugin::{RuleFormat, RuleSource};

#[cfg(feature = "apollo")]
#[test]
fn apollo_source_exposes_rule_contract_without_connecting() {
    let source = liteflow_rule_plugin::apollo::ApolloRuleSource::new(
        "127.0.0.1:8070",
        "sample",
        "default",
        "application",
    )
    .expect("Apollo 契约配置应有效");
    assert_eq!(source.name(), "apollo");
    assert_eq!(source.format(), RuleFormat::Xml);
}

#[cfg(feature = "etcd")]
#[test]
fn etcd_source_constructor_preserves_contract_metadata() {
    let source = liteflow_rule_plugin::etcd::EtcdRuleSource::new(
        vec!["http://127.0.0.1:2379".to_string()],
        "/liteflow/flow",
    )
    .expect("Etcd 契约配置应有效")
    .with_auth("user", "password")
    .expect("Etcd 认证配置应有效");
    assert_eq!(source.name(), "etcd");
    assert_eq!(source.format(), RuleFormat::Xml);
}

#[cfg(feature = "nacos")]
#[test]
fn nacos_source_constructor_preserves_contract_metadata() {
    let source = liteflow_rule_plugin::nacos::NacosRuleSource::new(
        "127.0.0.1:8848",
        "flow.xml",
        "DEFAULT_GROUP",
    )
    .expect("Nacos 契约配置应有效")
    .with_namespace("tenant")
    .expect("Nacos namespace 配置应有效")
    .with_auth("nacos", "nacos")
    .expect("Nacos 认证配置应有效");
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
    let config = liteflow_rule_plugin::zk::ZkParserVO::new("127.0.0.1:2181", "/lite-flow/flow");
    assert_eq!(config.connect_str(), "127.0.0.1:2181");
    assert_eq!(config.chain_path(), "/lite-flow/flow");
    assert!(config.validate().is_ok());
    // 构造解析器会建立真实 ZooKeeper 会话，离线契约只验证不会访问网络的 VO。
}
