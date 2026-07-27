//! Sharding JDBC 数据源连接器。

use rusqlite::Connection;

use super::super::{LiteFlowDataSourceConnect, LiteflowDataSourceConnectFactory};
use crate::parser::sql::{exception::ELSQLException, vo::SQLParserVO};

/// 按 `shardingJdbcDataSource` 名称打开已注册的 Rust 数据源。
///
/// 对应 Java:
/// `com.yomahub.liteflow.parser.sql.datasource.impl.ShardingJdbcDsConn`。
#[derive(Debug, Clone, Copy, Default)]
pub struct ShardingJdbcDsConn;

impl ShardingJdbcDsConn {
    /// Java 可选依赖类名，仅用于迁移诊断。
    pub const LOAD_CLASS_NAME: &'static str =
        "org.apache.shardingsphere.driver.jdbc.core.datasource.ShardingSphereDataSource";
    /// Java 可选依赖 groupId。
    pub const MAVEN_GROUP_ID: &'static str = "org.apache.shardingsphere";
    /// Java 可选依赖 artifactId。
    pub const MAVEN_ARTIFACT_ID: &'static str = "sharding-jdbc-core";
}

impl LiteFlowDataSourceConnect for ShardingJdbcDsConn {
    /// 配置了 Sharding 数据源名称时匹配。对应 Java `filter`。
    fn filter(&self, config: &SQLParserVO) -> Result<bool, ELSQLException> {
        Ok(config
            .sharding_jdbc_data_source
            .as_deref()
            .is_some_and(|name| !name.trim().is_empty()))
    }

    /// 打开同名注册数据源。对应 Java `getConn`。
    fn get_conn(&self, config: &SQLParserVO) -> Result<Connection, ELSQLException> {
        let name = config
            .sharding_jdbc_data_source
            .as_deref()
            .ok_or_else(|| ELSQLException::new("shardingJdbcDataSource is blank"))?;
        LiteflowDataSourceConnectFactory::open_data_source(name)
    }

    fn name(&self) -> &'static str {
        "ShardingJdbcDsConn"
    }
}
