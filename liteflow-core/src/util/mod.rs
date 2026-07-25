//! 对应 Java 包：com.yomahub.liteflow.util
//!
//! v2.10.0 基线 util 包含 8 个工具类（JsonUtil/SerialsUtil/CopyOnWriteHashMap 等），
//! 多数在 Rust 侧由标准库/serde/dashmap 直接覆盖，无需迁移；
//! 当前仅迁移 v2.16.0 新增的 ElRegexUtil（链继承占位符）。

pub mod el_regex;
