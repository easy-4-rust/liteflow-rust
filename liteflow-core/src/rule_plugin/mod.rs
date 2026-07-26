//! 外部规则源的 core 契约。
//!
//! 具体 Apollo/Etcd/Nacos/Redis/SQL/ZooKeeper 客户端已迁入独立
//! `liteflow-rule-plugin` crate；本模块只保留无外部 SDK 依赖的接口、格式枚举
//! 与 watcher，避免核心引擎被配置中心依赖拖入。

mod rule_fingerprint;
mod rule_format;
pub mod rule_source;
mod rule_source_watcher;

pub use rule_fingerprint::fnv_fp;
pub use rule_format::RuleFormat;
pub use rule_source::RuleSource;
pub use rule_source_watcher::RuleSourceWatcher;
