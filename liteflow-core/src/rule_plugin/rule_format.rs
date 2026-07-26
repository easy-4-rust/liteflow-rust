//! 规则文本格式。

/// 规则文本格式，对应 JSON/XML/YML 三类 parser。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleFormat {
    /// JSON 规则。
    Json,
    /// XML 规则。
    Xml,
    /// YAML 规则。
    Yml,
}
