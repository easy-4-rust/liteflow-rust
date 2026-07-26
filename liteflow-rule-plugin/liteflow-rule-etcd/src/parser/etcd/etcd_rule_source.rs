//! 对应 Java: `com.yomahub.liteflow.parser.etcd.EtcdXmlELParser`。

use async_trait::async_trait;
use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::rule_plugin::{RuleFormat, RuleSource, fnv_fp};
use tokio::sync::Mutex;

/// Etcd 规则源，基于官方 `etcd-client`。
pub struct EtcdRuleSource {
    pub endpoints: Vec<String>,
    pub key: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub format: RuleFormat,
    client: Mutex<Option<etcd_client::Client>>,
}

impl EtcdRuleSource {
    /// 创建规则源。对应 Java `EtcdClient` 初始化连接参数。
    pub fn new(endpoints: Vec<String>, key: impl Into<String>, format: RuleFormat) -> Self {
        Self {
            endpoints,
            key: key.into(),
            username: None,
            password: None,
            format,
            client: Mutex::new(None),
        }
    }

    /// 设置用户名密码。
    pub fn with_auth(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self.password = Some(password.into());
        self
    }

    async fn client(&self) -> LFResult<etcd_client::Client> {
        let mut guard = self.client.lock().await;
        if let Some(client) = guard.as_ref() {
            return Ok(client.clone());
        }
        let options = match (&self.username, &self.password) {
            (Some(username), Some(password)) => {
                Some(etcd_client::ConnectOptions::new().with_user(username, password))
            }
            _ => None,
        };
        let client = etcd_client::Client::connect(self.endpoints.clone(), options)
            .await
            .map_err(|error| LiteflowError::Rule(format!("etcd connect error: {error}")))?;
        *guard = Some(client.clone());
        Ok(client)
    }
}

#[async_trait]
impl RuleSource for EtcdRuleSource {
    /// 读取 key 内容。对应 Java `EtcdParserHelper#getContent`。
    async fn fetch(&self) -> LFResult<(String, String)> {
        let mut client = self.client().await?;
        let response = client
            .get(self.key.clone(), None)
            .await
            .map_err(|error| LiteflowError::Rule(format!("etcd get error: {error}")))?;
        let value = response
            .kvs()
            .first()
            .ok_or_else(|| LiteflowError::Rule(format!("etcd key[{}] not found", self.key)))?;
        let text = value
            .value_str()
            .map_err(|error| LiteflowError::Rule(format!("etcd decode error: {error}")))?
            .to_owned();
        Ok((text.clone(), fnv_fp(&text)))
    }

    fn format(&self) -> RuleFormat {
        self.format
    }

    fn name(&self) -> &str {
        "etcd"
    }
}
