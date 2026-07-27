//! Rust 通用规则源适配器。

use async_trait::async_trait;
use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::rule_plugin::{RuleFormat, RuleSource, fnv_fp};

use super::{ApolloParserConfigVO, ApolloXmlELParser};

/// 将 Java 对齐的 Apollo XML EL 解析器适配为 Rust `RuleSource`。
///
/// 该对象仅承担 Rust 基础设施适配，不替代或合并任何 Java 对象。
#[derive(Debug, Clone)]
pub struct ApolloRuleSource {
    parser: ApolloXmlELParser,
}

impl ApolloRuleSource {
    /// 创建只包含 Chain namespace 的 Apollo 规则源。
    ///
    /// 参数分别对应 Apollo Config Service 地址、应用 id、集群和 Chain namespace。
    pub fn new(
        config_service_url: impl Into<String>,
        app_id: impl Into<String>,
        cluster: impl Into<String>,
        chain_namespace: impl Into<String>,
    ) -> LFResult<Self> {
        let config = ApolloParserConfigVO::new(chain_namespace, None::<String>);
        let parser = ApolloXmlELParser::new(config, config_service_url, app_id, cluster)
            .map_err(LiteflowError::from)?;
        Ok(Self { parser })
    }

    /// 增加可选的 Script namespace。
    ///
    /// 参数 `script_namespace` 对应 Java `scriptNamespace`。
    pub fn with_script_namespace(mut self, script_namespace: impl Into<String>) -> LFResult<Self> {
        let mut config = self.parser.config().clone();
        config.set_script_namespace(Some(script_namespace.into()));
        self.parser = ApolloXmlELParser::new(
            config,
            self.parser.config_service_url(),
            self.parser.app_id(),
            self.parser.cluster(),
        )
        .map_err(LiteflowError::from)?
        .with_ip(self.parser.ip());
        Ok(self)
    }

    /// 设置 Config Service 请求中的客户端 IP。
    #[must_use]
    pub fn with_ip(mut self, ip: impl Into<String>) -> Self {
        self.parser = self.parser.with_ip(ip);
        self
    }

    /// 返回 Java 对齐的 Apollo 解析器。
    #[must_use]
    pub fn parser(&self) -> &ApolloXmlELParser {
        &self.parser
    }
}

#[async_trait]
impl RuleSource for ApolloRuleSource {
    /// 聚合 Chain/Script namespace 为 XML 并计算变更指纹。
    ///
    /// 对应 Java `ApolloXmlELParser#parseCustom`。
    async fn fetch(&self) -> LFResult<(String, String)> {
        let parser = self.parser.clone();
        let text = tokio::task::spawn_blocking(move || parser.parse_custom())
            .await
            .map_err(|error| LiteflowError::Rule(format!("apollo task error: {error}")))?
            .map_err(LiteflowError::from)?;
        Ok((text.clone(), fnv_fp(&text)))
    }

    fn format(&self) -> RuleFormat {
        RuleFormat::Xml
    }

    fn name(&self) -> &str {
        "apollo"
    }
}
