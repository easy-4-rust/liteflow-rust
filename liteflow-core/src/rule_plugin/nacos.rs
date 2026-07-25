//! 对应 liteflow-rule-nacos：基于 Nacos 官方 Rust SDK（nacos-sdk）。

use super::rule_source::{fnv_fp, RuleFormat, RuleSource};
use crate::exception::{LFResult, LiteflowError};
use async_trait::async_trait;
use nacos_sdk::api::config::{ConfigService, ConfigServiceBuilder};
use nacos_sdk::api::props::ClientProps;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Nacos 规则源（对应 NacosParser）
pub struct NacosRuleSource {
    pub server_addr: String,
    pub data_id: String,
    pub group: String,
    /// namespace（tenant）
    pub namespace: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub format: RuleFormat,
    service: Mutex<Option<Arc<Mutex<dyn ConfigService + Send>>>>,
}

impl NacosRuleSource {
    async fn service(&self) -> LFResult<Arc<Mutex<dyn ConfigService + Send>>> {
        let mut guard = self.service.lock().await;
        if let Some(s) = guard.as_ref() {
            return Ok(s.clone());
        }
        let mut props = ClientProps::new().server_addr(&self.server_addr);
        if let Some(ns) = &self.namespace {
            props = props.namespace(ns);
        }
        if let (Some(u), Some(p)) = (&self.username, &self.password) {
            props = props.auth_username(u).auth_password(p);
        }
        let service = ConfigServiceBuilder::new(props)
            .build()
            .map_err(|e| LiteflowError::Rule(format!("nacos client build error: {e}")))?;
        let service: Arc<Mutex<dyn ConfigService + Send>> = Arc::new(Mutex::new(service));
        *guard = Some(service.clone());
        Ok(service)
    }
}

#[async_trait]
impl RuleSource for NacosRuleSource {
    async fn fetch(&self) -> LFResult<(String, String)> {
        let service = self.service().await?;
        // nacos-sdk 0.3 的 ConfigService::get_config 为同步接口（内部阻塞 HTTP）
        let resp = service
            .lock()
            .await
            .get_config(self.data_id.clone(), self.group.clone())
            .map_err(|e| LiteflowError::Rule(format!("nacos get config error: {e}")))?;
        let text = resp.content().clone();
        let fp = fnv_fp(&text);
        Ok((text, fp))
    }
    fn format(&self) -> RuleFormat {
        self.format
    }
    fn name(&self) -> &str {
        "nacos"
    }
}
