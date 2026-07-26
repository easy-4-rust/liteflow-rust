//! 对应 Java LiteFlow 规则源支持的 XML/JSON/YML 格式。

use serde::{Deserialize, Serialize};

/// Vernal 配置中的规则文本格式。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LiteflowRuleFormat {
    /// JSON 规则。
    #[default]
    Json,
    /// XML 规则。
    Xml,
    /// YAML/YML 规则。
    Yml,
}
