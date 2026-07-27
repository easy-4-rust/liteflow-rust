//! Rust `RuleSource` 兼容入口。

use async_trait::async_trait;
use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::rule_plugin::{RuleFormat, RuleSource, fnv_fp};

use super::{SQLXmlELParser, vo::SQLParserVO};

/// 把 Java 对齐的 `SQLXmlELParser` 适配为 Rust 通用 `RuleSource`。
///
/// 该文件仅承载 Rust 基础设施适配，不合并 Java SQL 插件对象。对应 Java
/// 解析入口: `com.yomahub.liteflow.parser.sql.SQLXmlELParser`。
#[derive(Debug, Clone)]
pub struct SqlRuleSource {
    config: SQLParserVO,
}

impl SqlRuleSource {
    /// 使用本地 SQLite 路径创建兼容规则源。
    #[must_use]
    pub fn new(db_path: impl Into<String>) -> Self {
        Self {
            config: SQLParserVO::sqlite(db_path),
        }
    }

    /// 使用完整 Java 对齐配置创建规则源。
    #[must_use]
    pub fn from_config(config: SQLParserVO) -> Self {
        Self { config }
    }

    /// 返回规则源配置。
    #[must_use]
    pub fn config(&self) -> &SQLParserVO {
        &self.config
    }
}

#[async_trait]
impl RuleSource for SqlRuleSource {
    /// 读取 Chain/脚本表并生成 XML。对应 Java `SQLXmlELParser#parseCustom`。
    async fn fetch(&self) -> LFResult<(String, String)> {
        let parser = SQLXmlELParser::new(self.config.clone()).map_err(LiteflowError::from)?;
        let text = parser.parse_custom().map_err(LiteflowError::from)?;
        Ok((text.clone(), fnv_fp(&text)))
    }

    fn format(&self) -> RuleFormat {
        RuleFormat::Xml
    }

    fn name(&self) -> &str {
        "sql"
    }
}
