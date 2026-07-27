//! SQL XML EL 解析入口。

use liteflow_core::flow::flow_bus::FlowBus;

use super::{exception::ELSQLException, util::JDBCHelper, vo::SQLParserVO};

/// 校验 SQL 扩展配置并通过 `JDBCHelper` 生成 EL XML。
///
/// 只支持 EL XML，与 Java插件保持一致。对应 Java:
/// `com.yomahub.liteflow.parser.sql.SQLXmlELParser`。
#[derive(Debug, Clone)]
pub struct SQLXmlELParser {
    config: SQLParserVO,
    jdbc_helper: JDBCHelper,
}

impl SQLXmlELParser {
    /// 使用已经反序列化的扩展配置创建解析器。
    ///
    /// 对应 Java `SQLXmlELParser#SQLXmlELParser` 中配置解析后的初始化阶段。
    pub fn new(config: SQLParserVO) -> Result<Self, ELSQLException> {
        check_parser_vo(&config)?;
        let jdbc_helper = JDBCHelper::init(config.clone());
        Ok(Self {
            config,
            jdbc_helper,
        })
    }

    /// 从 Java/Jackson 兼容的 JSON 扩展数据创建解析器。
    pub fn from_json(rule_source_ext_data: &str) -> Result<Self, ELSQLException> {
        if rule_source_ext_data.trim().is_empty() {
            return Err(ELSQLException::new("rule-source-ext-data is empty"));
        }
        let config = serde_json::from_str(rule_source_ext_data)
            .map_err(|error| ELSQLException::new(error.to_string()))?;
        Self::new(config)
    }

    /// 读取数据库并返回完整 XML 规则。对应 Java `parseCustom()`。
    pub fn parse_custom(&self) -> Result<String, ELSQLException> {
        self.jdbc_helper.get_content()
    }

    /// 配置开启轮询时启动后台对账；未开启时返回 `None`。
    ///
    /// 对应 Java `parseCustom` 注册的 `FlowInitHook`。
    #[must_use]
    pub fn start_polling(&self, bus: FlowBus) -> Option<tokio::task::JoinHandle<()>> {
        self.config
            .polling_enabled
            .then(|| self.jdbc_helper.listen_sql(bus))
    }

    /// 返回 JDBC helper，供 instanceId 或测试复用同一配置。
    #[must_use]
    pub fn jdbc_helper(&self) -> &JDBCHelper {
        &self.jdbc_helper
    }

    /// 返回当前解析器配置。
    #[must_use]
    pub fn config(&self) -> &SQLParserVO {
        &self.config
    }
}

fn check_parser_vo(config: &SQLParserVO) -> Result<(), ELSQLException> {
    if config.is_auto_found_data_source() {
        return Ok(());
    }
    if config
        .url
        .as_deref()
        .is_some_and(|url| !url.trim().is_empty())
    {
        if config
            .driver_class_name
            .as_deref()
            .is_none_or(|driver| driver.trim().is_empty())
        {
            return Err(ELSQLException::new(
                "rule-source-ext-data driverClassName is blank",
            ));
        }
        if config.username.is_none() {
            return Err(ELSQLException::new(
                "rule-source-ext-data username is blank",
            ));
        }
        if config.password.is_none() {
            return Err(ELSQLException::new(
                "rule-source-ext-data password is blank",
            ));
        }
    }
    Ok(())
}
