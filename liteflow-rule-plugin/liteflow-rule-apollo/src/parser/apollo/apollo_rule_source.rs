//! 对应 Java: `com.yomahub.liteflow.parser.apollo.ApolloXmlELParser`。

use async_trait::async_trait;
use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::rule_plugin::{RuleFormat, RuleSource, fnv_fp};

/// Apollo 规则源。
///
/// Java 插件通过 Apollo Client SDK 监听 namespace；Rust 没有官方 SDK，
/// 使用 Apollo Config Service OpenAPI 拉取并交由通用 watcher 做变更检测。
pub struct ApolloRuleSource {
    pub portal_addr: String,
    pub app_id: String,
    pub cluster: String,
    pub namespace: String,
    /// 配置项 key，规则文本存放在 value 中。
    pub key: String,
    pub format: RuleFormat,
}

#[async_trait]
impl RuleSource for ApolloRuleSource {
    /// 拉取配置文本。对应 Java `ApolloParseHelper#getContent`。
    async fn fetch(&self) -> LFResult<(String, String)> {
        let url = format!(
            "http://{}/configfiles/json/{}/{}/{}?ip=rust",
            self.portal_addr, self.app_id, self.cluster, self.namespace
        );
        let key = self.key.clone();
        let text = tokio::task::spawn_blocking(move || -> LFResult<String> {
            let response: serde_json::Value = ureq::get(&url)
                .call()
                .map_err(|error| LiteflowError::Rule(format!("apollo fetch error: {error}")))?
                .body_mut()
                .read_json()
                .map_err(|error| LiteflowError::Rule(format!("apollo parse error: {error}")))?;
            response
                .get("configurations")
                .and_then(|configurations| configurations.get(&key))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| LiteflowError::Rule(format!("apollo key[{key}] not found")))
        })
        .await
        .map_err(|error| LiteflowError::Rule(format!("apollo task error: {error}")))??;
        Ok((text.clone(), fnv_fp(&text)))
    }

    fn format(&self) -> RuleFormat {
        self.format
    }

    fn name(&self) -> &str {
        "apollo"
    }
}
