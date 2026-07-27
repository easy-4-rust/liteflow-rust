//! 默认显式数据库连接器。

use rusqlite::Connection;

use super::super::LiteFlowDataSourceConnect;
use super::super::liteflow_data_source_connect_factory::normalize_sqlite_url;
use crate::parser::sql::{exception::ELSQLException, vo::SQLParserVO};

/// 使用配置中的 URL 创建 SQLite 连接。
///
/// 对应 Java:
/// `com.yomahub.liteflow.parser.sql.datasource.impl.DefaultLiteFlowJdbcConn`。
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultLiteFlowJdbcConn;

impl LiteFlowDataSourceConnect for DefaultLiteFlowJdbcConn {
    /// 显式 URL 与驱动配置齐全时匹配。对应 Java `filter`。
    fn filter(&self, config: &SQLParserVO) -> Result<bool, ELSQLException> {
        Ok(config.is_use_jdbc_conn())
    }

    /// 打开配置 URL 指向的 SQLite 数据库。对应 Java `getConn`。
    fn get_conn(&self, config: &SQLParserVO) -> Result<Connection, ELSQLException> {
        let url = config
            .url
            .as_deref()
            .ok_or_else(|| ELSQLException::new("rule-source-ext-data url is blank"))?;
        Connection::open(normalize_sqlite_url(url)).map_err(ELSQLException::from)
    }

    fn name(&self) -> &'static str {
        "DefaultLiteFlowJdbcConn"
    }
}
