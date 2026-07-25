//! 对应 liteflow-rule-plugin 各模块的 Rust 化，规则源使用对应框架的官方/主流
//! Rust SDK 实现，按 cargo feature 启用：
//! - `nacos`：nacos-sdk（Nacos 官方 Rust SDK）
//! - `etcd`：etcd-client（官方）
//! - `zk`：zookeeper crate
//! - `apollo`：Apollo 官方 Open API（无官方 Rust 客户端，HTTP 实现）
//! - `redis`：redis crate
//! - `sql`：rusqlite（对应 liteflow-rule-sql 的表结构）

pub mod rule_source;
#[cfg(feature = "nacos")]
pub mod nacos;
#[cfg(feature = "apollo")]
pub mod apollo;
#[cfg(feature = "etcd")]
pub mod etcd;
#[cfg(feature = "zk")]
pub mod zk;
#[cfg(feature = "redis")]
pub mod redis_source;
#[cfg(feature = "sql")]
pub mod sql_source;

pub use rule_source::{fnv_fp, RuleFormat, RuleSource, RuleSourceWatcher};
