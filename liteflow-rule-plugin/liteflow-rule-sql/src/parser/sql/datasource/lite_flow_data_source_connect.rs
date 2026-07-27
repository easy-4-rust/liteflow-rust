//! SQL 数据源连接契约。

use rusqlite::Connection;

use crate::parser::sql::{exception::ELSQLException, vo::SQLParserVO};

/// 按配置判断适用性并创建数据库连接。
///
/// Rust 以 `rusqlite::Connection` 映射 JDBC `Connection`，自定义实现可注册到工厂
/// 并优先于内置连接器执行。对应 Java:
/// `com.yomahub.liteflow.parser.sql.datasource.LiteFlowDataSourceConnect`。
pub trait LiteFlowDataSourceConnect: Send + Sync + 'static {
    /// 检查连接器是否支持该配置。对应 Java `filter(SQLParserVO)`。
    fn filter(&self, config: &SQLParserVO) -> Result<bool, ELSQLException>;

    /// 获取数据库连接。对应 Java `getConn(SQLParserVO)`。
    fn get_conn(&self, config: &SQLParserVO) -> Result<Connection, ELSQLException>;

    /// 返回用于诊断的连接器名称。
    fn name(&self) -> &'static str;
}
