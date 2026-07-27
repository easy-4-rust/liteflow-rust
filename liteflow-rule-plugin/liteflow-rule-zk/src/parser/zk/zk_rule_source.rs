//! Rust 通用规则源适配器。

use async_trait::async_trait;
use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::rule_plugin::{RuleFormat, RuleSource, fnv_fp};

use super::{ZkParserVO, ZkXmlELParser};

/// 将 Java 对齐的 ZooKeeper XML EL 解析器适配为 Rust `RuleSource`。
///
/// 该对象只承担 Rust 基础设施适配，不替代 Java ZooKeeper 插件的 5 个对象。
#[derive(Clone)]
pub struct ZkRuleSource {
    parser: ZkXmlELParser,
}

impl ZkRuleSource {
    /// 使用完整 Java 对齐配置创建 ZooKeeper 规则源。
    pub fn from_config(config: ZkParserVO) -> LFResult<Self> {
        let parser = ZkXmlELParser::new(config).map_err(LiteflowError::from)?;
        Ok(Self { parser })
    }

    /// 使用连接串和 Chain 根路径创建 ZooKeeper 规则源。
    pub fn new(connect_str: impl Into<String>, chain_path: impl Into<String>) -> LFResult<Self> {
        Self::from_config(ZkParserVO::new(connect_str, chain_path))
    }

    /// 设置可选 Script 根路径。
    pub fn with_script_path(mut self, script_path: impl Into<String>) -> LFResult<Self> {
        let mut config = self.parser.config().clone();
        config.set_script_path(Some(script_path.into()));
        self.parser = ZkXmlELParser::new(config).map_err(LiteflowError::from)?;
        Ok(self)
    }

    /// 返回 Java 对齐的 ZooKeeper 解析器。
    #[must_use]
    pub fn parser(&self) -> &ZkXmlELParser {
        &self.parser
    }
}

#[async_trait]
impl RuleSource for ZkRuleSource {
    /// 聚合 Chain/Script 子节点为 XML。对应 Java `ZkXmlELParser#parseCustom`。
    async fn fetch(&self) -> LFResult<(String, String)> {
        let parser = self.parser.clone();
        let text = tokio::task::spawn_blocking(move || parser.parse_custom())
            .await
            .map_err(|error| LiteflowError::Rule(format!("zk task error: {error}")))??;
        Ok((text.clone(), fnv_fp(&text)))
    }

    fn format(&self) -> RuleFormat {
        RuleFormat::Xml
    }

    fn name(&self) -> &str {
        "zookeeper"
    }
}
