//! 对应 Java: `com.yomahub.liteflow.parser.nacos.NacosXmlELParser`。

use async_trait::async_trait;
use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::rule_plugin::{RuleFormat, RuleSource, fnv_fp};
use nacos_sdk::api::config::{ConfigService, ConfigServiceBuilder};
use nacos_sdk::api::props::ClientProps;
use tokio::sync::Mutex;

/// Nacos 规则源，基于 Nacos 官方 Rust SDK。
pub struct NacosRuleSource {
    pub server_addr: String,
    pub data_id: String,
    pub group: String,
    /// namespace（tenant）。
    pub namespace: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub format: RuleFormat,
    service: Mutex<Option<ConfigService>>,
}

impl NacosRuleSource {
    /// 创建 Nacos 规则源。对应 Java `NacosParserVO` 的必要配置。
    pub fn new(
        server_addr: impl Into<String>,
        data_id: impl Into<String>,
        group: impl Into<String>,
        format: RuleFormat,
    ) -> Self {
        Self {
            server_addr: server_addr.into(),
            data_id: data_id.into(),
            group: group.into(),
            namespace: None,
            username: None,
            password: None,
            format,
            service: Mutex::new(None),
        }
    }

    /// 设置 namespace。
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// 设置用户名密码。
    pub fn with_auth(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self.password = Some(password.into());
        self
    }

    async fn service(&self) -> LFResult<ConfigService> {
        let mut guard = self.service.lock().await;
        if let Some(service) = guard.as_ref() {
            return Ok(service.clone());
        }
        let mut properties = ClientProps::new().server_addr(&self.server_addr);
        if let Some(namespace) = &self.namespace {
            properties = properties.namespace(namespace);
        }
        if let (Some(username), Some(password)) = (&self.username, &self.password) {
            properties = properties.auth_username(username).auth_password(password);
        }
        let service = ConfigServiceBuilder::new(properties)
            .build()
            .await
            .map_err(|error| LiteflowError::Rule(format!("nacos client build error: {error}")))?;
        *guard = Some(service.clone());
        Ok(service)
    }
}

#[async_trait]
impl RuleSource for NacosRuleSource {
    /// 拉取 dataId/group 内容。对应 Java `NacosParserHelper#getContent`。
    async fn fetch(&self) -> LFResult<(String, String)> {
        let response = self
            .service()
            .await?
            .get_config(self.data_id.clone(), self.group.clone())
            .await
            .map_err(|error| LiteflowError::Rule(format!("nacos get config error: {error}")))?;
        let text = response.content().clone();
        Ok((text.clone(), fnv_fp(&text)))
    }

    fn format(&self) -> RuleFormat {
        self.format
    }

    fn name(&self) -> &str {
        "nacos"
    }
}
