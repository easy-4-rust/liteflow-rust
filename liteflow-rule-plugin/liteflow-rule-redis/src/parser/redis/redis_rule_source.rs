//! 对应 Java: `com.yomahub.liteflow.parser.redis.RedisParser`。

use async_trait::async_trait;
use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::rule_plugin::{RuleFormat, RuleSource, fnv_fp};

/// Redis 单 key 规则源。
///
/// Java 插件支持订阅/集群配置；Rust 以 URL 交给 redis crate，服务器拓扑由
/// URL/客户端负责，通用 watcher 承担周期性变更检测。
pub struct RedisRuleSource {
    pub url: String,
    pub key: String,
    pub format: RuleFormat,
}

#[async_trait]
impl RuleSource for RedisRuleSource {
    /// GET 规则 key。对应 Java `RedisParser#getContent`。
    async fn fetch(&self) -> LFResult<(String, String)> {
        let url = self.url.clone();
        let key = self.key.clone();
        let text = tokio::task::spawn_blocking(move || -> LFResult<String> {
            let client = redis::Client::open(url.as_str())
                .map_err(|error| LiteflowError::Rule(format!("redis open error: {error}")))?;
            let mut connection = client
                .get_connection()
                .map_err(|error| LiteflowError::Rule(format!("redis connect error: {error}")))?;
            redis::Commands::get(&mut connection, &key)
                .map_err(|error| LiteflowError::Rule(format!("redis get error: {error}")))
        })
        .await
        .map_err(|error| LiteflowError::Rule(format!("redis task error: {error}")))??;
        Ok((text.clone(), fnv_fp(&text)))
    }

    fn format(&self) -> RuleFormat {
        self.format
    }

    fn name(&self) -> &str {
        "redis"
    }
}
