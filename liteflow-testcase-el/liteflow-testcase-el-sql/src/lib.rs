//! SQL 规则插件本地 SQLite 契约场景。

use liteflow_core::rule_plugin::{RuleFormat, RuleSource};
use liteflow_rule_sql::SqlRuleSource;

/// 构建本地 SQLite 规则源。
pub async fn run_case() -> bool {
    let source = SqlRuleSource::new(":memory:");
    source.name() == "sql" && source.format() == RuleFormat::Json
}
