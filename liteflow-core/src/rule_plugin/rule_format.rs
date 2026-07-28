//! 规则文本格式。

/// 规则文本格式。
///
/// 这是 Rust 规则插件统一分派 JSON/XML/YML parser 的类型安全枚举，不对应
/// 独立 Java 对象。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleFormat {
    /// JSON 规则。
    Json,
    /// XML 规则。
    Xml,
    /// YAML 规则。
    Yml,
}
