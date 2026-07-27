//! Rust 通用规则源适配器。

use async_trait::async_trait;
use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::rule_plugin::{RuleFormat, RuleSource, fnv_fp};

use super::{EtcdParserVO, EtcdXmlELParser};

/// 将 Java 对齐的 Etcd XML EL 解析器适配为 Rust `RuleSource`。
///
/// 该对象只承担 Rust 基础设施适配，不计入 Java Etcd 插件的 6 个对象。
#[derive(Debug, Clone)]
pub struct EtcdRuleSource {
    parser: EtcdXmlELParser,
}

impl EtcdRuleSource {
    /// 使用完整 Java 对齐配置创建规则源。
    pub fn from_config(config: EtcdParserVO) -> LFResult<Self> {
        let parser = EtcdXmlELParser::new(config).map_err(LiteflowError::from)?;
        Ok(Self { parser })
    }

    /// 使用 endpoints 和 Chain 路径创建规则源。
    pub fn new(endpoints: Vec<String>, chain_path: impl Into<String>) -> LFResult<Self> {
        Self::from_config(EtcdParserVO::new(endpoints.join(","), chain_path))
    }

    /// 设置可选 Script 路径。
    pub fn with_script_path(mut self, script_path: impl Into<String>) -> LFResult<Self> {
        let mut config = self.parser.config().clone();
        config.set_script_path(Some(script_path.into()));
        self.parser = EtcdXmlELParser::new(config).map_err(LiteflowError::from)?;
        Ok(self)
    }

    /// 设置 namespace 前缀。
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> LFResult<Self> {
        let mut config = self.parser.config().clone();
        config.set_namespace(Some(namespace.into()));
        self.parser = EtcdXmlELParser::new(config).map_err(LiteflowError::from)?;
        Ok(self)
    }

    /// 设置用户名密码。
    pub fn with_auth(
        mut self,
        user: impl Into<String>,
        password: impl Into<String>,
    ) -> LFResult<Self> {
        let mut config = self.parser.config().clone();
        config.set_user(Some(user.into()));
        config.set_password(Some(password.into()));
        self.parser = EtcdXmlELParser::new(config).map_err(LiteflowError::from)?;
        Ok(self)
    }

    /// 返回 Java 对齐的 Etcd 解析器。
    #[must_use]
    pub fn parser(&self) -> &EtcdXmlELParser {
        &self.parser
    }
}

#[async_trait]
impl RuleSource for EtcdRuleSource {
    /// 读取前缀树并聚合为 XML。对应 Java `EtcdXmlELParser#parseCustom`。
    async fn fetch(&self) -> LFResult<(String, String)> {
        let text = self
            .parser
            .parse_custom()
            .await
            .map_err(LiteflowError::from)?;
        Ok((text.clone(), fnv_fp(&text)))
    }

    fn format(&self) -> RuleFormat {
        RuleFormat::Xml
    }

    fn name(&self) -> &str {
        "etcd"
    }
}
