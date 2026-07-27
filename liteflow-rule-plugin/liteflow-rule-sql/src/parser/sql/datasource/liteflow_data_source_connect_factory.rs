//! 数据源连接器工厂。

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock, RwLock};

use rusqlite::Connection;

use super::LiteFlowDataSourceConnect;
use super::impls::{
    BaoMiDouDynamicDsConn, DefaultLiteFlowJdbcConn, LiteFlowAutoLookUpJdbcConn, ShardingJdbcDsConn,
};
use crate::parser::sql::{exception::ELSQLException, vo::SQLParserVO};

static CUSTOM_CONNECTORS: OnceLock<RwLock<Vec<Arc<dyn LiteFlowDataSourceConnect>>>> =
    OnceLock::new();
static DATA_SOURCES: OnceLock<RwLock<BTreeMap<String, String>>> = OnceLock::new();

/// 管理自定义连接器和 Rust 运行期命名数据源。
///
/// 连接器优先级保持 Java 顺序：自定义、显式 JDBC、苞米豆名称、
/// Sharding 名称、自动查找。对应 Java:
/// `com.yomahub.liteflow.parser.sql.datasource.LiteflowDataSourceConnectFactory`。
pub struct LiteflowDataSourceConnectFactory;

impl LiteflowDataSourceConnectFactory {
    /// 注册自定义连接器；后注册对象保持注册顺序并优先于内置实现。
    ///
    /// 对应 Java `register()` 从 IoC 容器读取自定义 Bean 的行为。
    pub fn register(connect: Arc<dyn LiteFlowDataSourceConnect>) {
        custom_connectors()
            .write()
            .expect("SQL 自定义连接器写锁中毒")
            .push(connect);
    }

    /// 注册一个命名 SQLite 数据源，供动态数据源和自动查找连接器使用。
    ///
    /// Rust 使用显式注册表替代 Spring `DataSource` Bean 查询。
    pub fn register_data_source(name: impl Into<String>, path: impl Into<String>) {
        data_sources()
            .write()
            .expect("SQL 数据源注册表写锁中毒")
            .insert(name.into(), path.into());
    }

    /// 返回命名数据源路径。
    #[must_use]
    pub fn data_source_path(name: &str) -> Option<String> {
        data_sources()
            .read()
            .expect("SQL 数据源注册表读锁中毒")
            .get(name)
            .cloned()
    }

    /// 返回当前命名数据源快照，供自动查找逐个探测。
    #[must_use]
    pub fn data_sources() -> BTreeMap<String, String> {
        data_sources()
            .read()
            .expect("SQL 数据源注册表读锁中毒")
            .clone()
    }

    /// 打开命名数据源。
    pub fn open_data_source(name: &str) -> Result<Connection, ELSQLException> {
        let path = Self::data_source_path(name)
            .ok_or_else(|| ELSQLException::new(format!("can not found {name} datasource")))?;
        Connection::open(normalize_sqlite_url(&path)).map_err(ELSQLException::from)
    }

    /// 按 Java 优先级选择首个匹配连接器。
    ///
    /// 对应 Java `LiteflowDataSourceConnectFactory#getConnect`。
    pub fn get_connect(
        config: &SQLParserVO,
    ) -> Result<Arc<dyn LiteFlowDataSourceConnect>, ELSQLException> {
        let mut connects = custom_connectors()
            .read()
            .expect("SQL 自定义连接器读锁中毒")
            .clone();
        connects.push(Arc::new(DefaultLiteFlowJdbcConn));
        connects.push(Arc::new(BaoMiDouDynamicDsConn));
        connects.push(Arc::new(ShardingJdbcDsConn));
        connects.push(Arc::new(LiteFlowAutoLookUpJdbcConn));

        for connect in connects {
            if connect.filter(config)? {
                return Ok(connect);
            }
        }
        Err(ELSQLException::new(
            "can not found connect by liteflow config",
        ))
    }
}

pub(crate) fn normalize_sqlite_url(url: &str) -> &str {
    url.strip_prefix("jdbc:sqlite:")
        .or_else(|| url.strip_prefix("sqlite://"))
        .unwrap_or(url)
}

fn custom_connectors() -> &'static RwLock<Vec<Arc<dyn LiteFlowDataSourceConnect>>> {
    CUSTOM_CONNECTORS.get_or_init(|| RwLock::new(Vec::new()))
}

fn data_sources() -> &'static RwLock<BTreeMap<String, String>> {
    DATA_SOURCES.get_or_init(|| RwLock::new(BTreeMap::new()))
}
