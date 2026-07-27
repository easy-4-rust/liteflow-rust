//! Rust 通用规则源适配器。

use async_trait::async_trait;
use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::rule_plugin::{RuleFormat, RuleSource, fnv_fp};

use super::{NacosParserVO, NacosXmlELParser};

/// 将 Java 对齐的 Nacos XML EL 解析器适配为 Rust `RuleSource`。
///
/// 该对象只承担 Rust 基础设施适配，不替代 Java Nacos 插件的 5 个对象。
#[derive(Debug, Clone)]
pub struct NacosRuleSource {
    parser: NacosXmlELParser,
}

impl NacosRuleSource {
    /// 使用完整 Java 对齐配置创建 Nacos 规则源。
    pub fn from_config(config: NacosParserVO) -> LFResult<Self> {
        let parser = NacosXmlELParser::new(config).map_err(LiteflowError::from)?;
        Ok(Self { parser })
    }

    /// 使用服务地址、dataId 与 group 创建 Nacos 规则源。
    ///
    /// Nacos Java 插件只支持 XML EL，因此不再接受任意规则格式。
    pub fn new(
        server_addr: impl Into<String>,
        data_id: impl Into<String>,
        group: impl Into<String>,
    ) -> LFResult<Self> {
        let mut config = NacosParserVO::default();
        config.set_server_addr(server_addr);
        config.set_data_id(data_id);
        config.set_group(group);
        Self::from_config(config)
    }

    /// 设置 namespace（tenant）。
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> LFResult<Self> {
        let mut config = self.parser.config().clone();
        config.set_namespace(namespace);
        self.parser = NacosXmlELParser::new(config).map_err(LiteflowError::from)?;
        Ok(self)
    }

    /// 设置用户名密码。
    pub fn with_auth(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> LFResult<Self> {
        let mut config = self.parser.config().clone();
        config.set_username(username);
        config.set_password(password);
        self.parser = NacosXmlELParser::new(config).map_err(LiteflowError::from)?;
        Ok(self)
    }

    /// 设置 AccessKey 与 SecretKey。
    pub fn with_access_key(
        mut self,
        access_key: impl Into<String>,
        secret_key: impl Into<String>,
    ) -> LFResult<Self> {
        let mut config = self.parser.config().clone();
        config.set_access_key(access_key);
        config.set_secret_key(secret_key);
        self.parser = NacosXmlELParser::new(config).map_err(LiteflowError::from)?;
        Ok(self)
    }

    /// 返回 Java 对齐的 Nacos 解析器。
    #[must_use]
    pub fn parser(&self) -> &NacosXmlELParser {
        &self.parser
    }
}

#[async_trait]
impl RuleSource for NacosRuleSource {
    /// 拉取并校验 dataId/group 内容。对应 Java `NacosXmlELParser#parseCustom`。
    async fn fetch(&self) -> LFResult<(String, String)> {
        let text = self.parser.parse_custom().await?;
        Ok((text.clone(), fnv_fp(&text)))
    }

    fn format(&self) -> RuleFormat {
        RuleFormat::Xml
    }

    fn name(&self) -> &str {
        "nacos"
    }
}
