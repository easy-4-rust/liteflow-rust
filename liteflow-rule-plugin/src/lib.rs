//! LiteFlow 外部规则源插件。
//!
//! 对应 Java `liteflow-rule-plugin` 聚合模块。每种规则源位于独立子 crate，
//! 本 crate 只按 Cargo feature 重导出，不承载具体规则源对象。

#[cfg(feature = "apollo")]
pub use liteflow_rule_apollo as apollo;
#[cfg(feature = "etcd")]
pub use liteflow_rule_etcd as etcd;
#[cfg(feature = "nacos")]
pub use liteflow_rule_nacos as nacos;
#[cfg(feature = "redis")]
pub use liteflow_rule_redis as redis;
#[cfg(feature = "sql")]
pub use liteflow_rule_sql as sql;
#[cfg(feature = "zk")]
pub use liteflow_rule_zk as zk;

pub use liteflow_core::rule_plugin::{RuleFormat, RuleSource, RuleSourceWatcher, fnv_fp};
