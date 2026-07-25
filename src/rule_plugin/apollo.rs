//! 对应 liteflow-rule-apollo：Apollo 官方 Open API（无官方 Rust 客户端）。

use super::rule_source::{fnv_fp, RuleFormat, RuleSource};
use crate::exception::{LFResult, LiteflowError};
use async_trait::async_trait;

/// Apollo 规则源（对应 ApolloParser）
pub struct ApolloRuleSource {
    pub portal_addr: String,
    pub app_id: String,
    pub cluster: String,
    pub namespace: String,
    /// 配置项 key（规则文本存放在 value 中）
    pub key: String,
    pub format: RuleFormat,
}

#[async_trait]
impl RuleSource for ApolloRuleSource {
    async fn fetch(&self) -> LFResult<(String, String)> {
        let url = format!(
            "http://{}/configfiles/json/{}/{}/{}?ip=rust",
            self.portal_addr, self.app_id, self.cluster, self.namespace
        );
        let key = self.key.clone();
        let text = tokio::task::spawn_blocking(move || -> LFResult<String> {
            let resp: serde_json::Value = ureq::get(&url)
                .call()
                .map_err(|e| LiteflowError::Rule(format!("apollo fetch error: {e}")))?
                .body_mut()
                .read_json()
                .map_err(|e| LiteflowError::Rule(format!("apollo parse error: {e}")))?;
            resp.get("configurations")
                .and_then(|c| c.get(&key))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| LiteflowError::Rule("apollo key not found".into()))
        })
        .await
        .map_err(|e| LiteflowError::Rule(format!("apollo task error: {e}")))??;
        let fp = fnv_fp(&text);
        Ok((text, fp))
    }
    fn format(&self) -> RuleFormat {
        self.format
    }
    fn name(&self) -> &str {
        "apollo"
    }
}
