//! 自动查找数据源连接器。

use std::sync::{OnceLock, RwLock};

use rusqlite::Connection;

use super::super::{LiteFlowDataSourceConnect, LiteflowDataSourceConnectFactory};
use crate::parser::sql::{exception::ELSQLException, util::LiteFlowJdbcUtil, vo::SQLParserVO};

static DATA_SOURCE_NAME: OnceLock<RwLock<Option<String>>> = OnceLock::new();

/// 遍历命名数据源并缓存首个可执行 LiteFlow 检查 SQL 的数据源。
///
/// 对应 Java:
/// `com.yomahub.liteflow.parser.sql.datasource.impl.LiteFlowAutoLookUpJdbcConn`。
#[derive(Debug, Clone, Copy, Default)]
pub struct LiteFlowAutoLookUpJdbcConn;

impl LiteFlowAutoLookUpJdbcConn {
    /// 返回自动探测并缓存的数据源名称。
    #[must_use]
    pub fn data_source_name() -> Option<String> {
        data_source_name()
            .read()
            .expect("SQL 自动数据源名称读锁中毒")
            .clone()
    }

    /// 自动查找包含 LiteFlow Chain 表的数据源。对应 Java `autoLookUpConn`。
    pub fn auto_look_up_conn(config: &SQLParserVO) -> Result<Connection, ELSQLException> {
        if let Some(name) = Self::data_source_name() {
            return LiteflowDataSourceConnectFactory::open_data_source(&name);
        }

        let check_sql = LiteFlowJdbcUtil::build_check_sql(config)?;
        for (name, path) in LiteflowDataSourceConnectFactory::data_sources() {
            let Ok(connection) = Connection::open(path) else {
                continue;
            };
            if LiteFlowJdbcUtil::check_connection_can_execute_sql(&connection, &check_sql) {
                *data_source_name()
                    .write()
                    .expect("SQL 自动数据源名称写锁中毒") = Some(name.clone());
                return Ok(connection);
            }
        }
        Err(ELSQLException::new(format!(
            "can not found liteflow config in dataSourceName {:?}",
            LiteflowDataSourceConnectFactory::data_sources()
                .keys()
                .collect::<Vec<_>>()
        )))
    }
}

impl LiteFlowDataSourceConnect for LiteFlowAutoLookUpJdbcConn {
    /// 自动查找连接器始终作为最后兜底匹配。对应 Java `filter`。
    fn filter(&self, _config: &SQLParserVO) -> Result<bool, ELSQLException> {
        Ok(true)
    }

    /// 自动查找并打开数据库。对应 Java `getConn`。
    fn get_conn(&self, config: &SQLParserVO) -> Result<Connection, ELSQLException> {
        Self::auto_look_up_conn(config)
    }

    fn name(&self) -> &'static str {
        "LiteFlowAutoLookUpJdbcConn"
    }
}

fn data_source_name() -> &'static RwLock<Option<String>> {
    DATA_SOURCE_NAME.get_or_init(|| RwLock::new(None))
}
