//! SQL 连接与探测工具。

use rusqlite::Connection;

use crate::parser::sql::{
    datasource::LiteflowDataSourceConnectFactory, exception::ELSQLException, vo::SQLParserVO,
};

/// 统一完成连接器选择、连接可用性检查和检查 SQL 构造。
///
/// Rust 连接通过 RAII 自动关闭，不需要 Java 的显式 `close` 重载。对应 Java:
/// `com.yomahub.liteflow.parser.sql.util.LiteFlowJdbcUtil`。
pub struct LiteFlowJdbcUtil;

impl LiteFlowJdbcUtil {
    /// 按连接器优先级获取连接。
    ///
    /// 对应 Java `LiteFlowJdbcUtil#getConn`。
    pub fn get_conn(config: &SQLParserVO) -> Result<Connection, ELSQLException> {
        let connect = LiteflowDataSourceConnectFactory::get_connect(config)?;
        connect.get_conn(config)
    }

    /// 判断连接能否执行指定 SQL。
    ///
    /// 参数 `conn` 为现有连接，`sql` 为只读探测语句。对应 Java
    /// `LiteFlowJdbcUtil#checkConnectionCanExecuteSql`。
    #[must_use]
    pub fn check_connection_can_execute_sql(conn: &Connection, sql: &str) -> bool {
        let Ok(mut statement) = conn.prepare(sql) else {
            return false;
        };
        // Java `executeQuery()` 在零行结果时仍表示 SQL 可执行，不能使用
        // `query_row`，否则空表会被误判为不可用数据源。
        statement.query([]).is_ok()
    }

    /// 构建自动数据源探测 SQL。
    ///
    /// 对应 Java `LiteFlowJdbcUtil#buildCheckSql`。
    pub fn build_check_sql(config: &SQLParserVO) -> Result<String, ELSQLException> {
        let table = required(
            config.chain_table_name.as_deref(),
            "chainTableName is blank",
        )?;
        if config.chain_name_field.trim().is_empty() {
            return Err(ELSQLException::new("chainNameField is blank"));
        }
        if config.el_data_field.trim().is_empty() {
            return Err(ELSQLException::new("elDataField is blank"));
        }
        Ok(format!(
            "SELECT {},{} FROM {} LIMIT 1",
            config.chain_name_field, config.el_data_field, table
        ))
    }
}

fn required<'a>(value: Option<&'a str>, message: &'static str) -> Result<&'a str, ELSQLException> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ELSQLException::new(message))
}
