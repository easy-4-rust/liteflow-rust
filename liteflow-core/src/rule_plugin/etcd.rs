//! 对应 liteflow-rule-etcd：基于官方 etcd-client crate。

use super::rule_source::{fnv_fp, RuleFormat, RuleSource};
use crate::exception::{LFResult, LiteflowError};
use async_trait::async_trait;
use tokio::sync::Mutex;

/// Etcd 规则源（对应 EtcdParser）
pub struct EtcdRuleSource {
    pub endpoints: Vec<String>,
    pub key: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub format: RuleFormat,
    client: Mutex<Option<etcd_client::Client>>,
}

impl EtcdRuleSource {
    async fn client(&self) -> LFResult<etcd_client::Client> {
        let mut guard = self.client.lock().await;
        if let Some(c) = guard.as_ref() {
            return Ok(c.clone());
        }
        let options = match (&self.username, &self.password) {
            (Some(u), Some(p)) => Some(etcd_client::ConnectOptions::new().with_user(u, p)),
            _ => None,
        };
        let client = etcd_client::Client::connect(self.endpoints.clone(), options)
            .await
            .map_err(|e| LiteflowError::Rule(format!("etcd connect error: {e}")))?;
        *guard = Some(client.clone());
        Ok(client)
    }
}

#[async_trait]
impl RuleSource for EtcdRuleSource {
    async fn fetch(&self) -> LFResult<(String, String)> {
        let mut client = self.client().await?;
        let resp = client
            .get(self.key.clone(), None)
            .await
            .map_err(|e| LiteflowError::Rule(format!("etcd get error: {e}")))?;
        let kv = resp
            .kvs()
            .first()
            .ok_or_else(|| LiteflowError::Rule(format!("etcd key[{}] not found", self.key)))?;
        let text = kv
            .value_str()
            .map_err(|e| LiteflowError::Rule(format!("etcd decode error: {e}")))?
            .to_string();
        let fp = fnv_fp(&text);
        Ok((text, fp))
    }
    fn format(&self) -> RuleFormat {
        self.format
    }
    fn name(&self) -> &str {
        "etcd"
    }
}
