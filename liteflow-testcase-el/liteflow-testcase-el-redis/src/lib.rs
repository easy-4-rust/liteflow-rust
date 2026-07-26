//! Redis 规则插件离线契约场景。

use liteflow_core::rule_plugin::{RuleFormat, RuleSource};
use liteflow_rule_redis::RedisRuleSource;

/// 构建 Redis 规则源而不访问外部服务。
pub async fn run_case() -> bool {
    let source = RedisRuleSource {
        url: "redis://redis.example.test/".to_string(),
        key: "liteflow:flow".to_string(),
        format: RuleFormat::Yml,
    };
    source.name() == "redis" && source.format() == RuleFormat::Yml
}
