//! 对应 liteflow-rule-redis：基于 redis crate（GET key）。

use super::rule_source::{fnv_fp, RuleFormat, RuleSource};
use crate::exception::{LFResult, LiteflowError};
use async_trait::async_trait;

/// Redis 规则源（对应 RedisParser；单机直连模式）
pub struct RedisRuleSource {
    pub url: String,
    pub key: String,
    pub format: RuleFormat,
}

#[async_trait]
impl RuleSource for RedisRuleSource {
    async fn fetch(&self) -> LFResult<(String, String)> {
        let url = self.url.clone();
        let key = self.key.clone();
        let text = tokio::task::spawn_blocking(move || -> LFResult<String> {
            let client = redis::Client::open(url.as_str())
                .map_err(|e| LiteflowError::Rule(format!("redis open error: {e}")))?;
            let mut conn = client
                .get_connection()
                .map_err(|e| LiteflowError::Rule(format!("redis connect error: {e}")))?;
            redis::Commands::get(&mut conn, &key)
                .map_err(|e| LiteflowError::Rule(format!("redis get error: {e}")))
        })
        .await
        .map_err(|e| LiteflowError::Rule(format!("redis task error: {e}")))??;
        let fp = fnv_fp(&text);
        Ok((text, fp))
    }
    fn format(&self) -> RuleFormat {
        self.format
    }
    fn name(&self) -> &str {
        "redis"
    }
}
