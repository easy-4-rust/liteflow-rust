//! Vernal 配置集成场景。

use liteflow_vernal::{LiteflowConfig, LiteflowRuleFormat};

/// 校验内联 XML 规则可通过 Vernal 类型安全配置表达。
pub async fn run_case() -> bool {
    let config = LiteflowConfig::new().with_inline_rule(
        LiteflowRuleFormat::Xml,
        "<flow><chain name=\"vernal\">THEN(a)</chain></flow>",
    );
    config.enable && config.inline_rule.is_some()
}
