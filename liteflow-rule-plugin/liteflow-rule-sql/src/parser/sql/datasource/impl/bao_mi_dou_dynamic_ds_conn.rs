//! 苞米豆动态数据源连接器。

use rusqlite::Connection;

use super::super::{LiteFlowDataSourceConnect, LiteflowDataSourceConnectFactory};
use crate::parser::sql::{exception::ELSQLException, vo::SQLParserVO};

/// 按 `baomidouDataSource` 名称从 Rust 数据源注册表取得连接。
///
/// 这是 Java DynamicRoutingDataSource 的 Rust 原生映射。对应 Java:
/// `com.yomahub.liteflow.parser.sql.datasource.impl.BaoMiDouDynamicDsConn`。
#[derive(Debug, Clone, Copy, Default)]
pub struct BaoMiDouDynamicDsConn;

impl BaoMiDouDynamicDsConn {
    /// Java 可选依赖类名，仅用于迁移诊断。
    pub const LOAD_CLASS_NAME: &'static str =
        "com.baomidou.dynamic.datasource.DynamicRoutingDataSource";
    /// Java 可选依赖 groupId。
    pub const MAVEN_GROUP_ID: &'static str = "com.baomidou";
    /// Java 可选依赖 artifactId。
    pub const MAVEN_ARTIFACT_ID: &'static str = "dynamic-datasource-spring-boot-starter";
}

impl LiteFlowDataSourceConnect for BaoMiDouDynamicDsConn {
    /// 配置了苞米豆数据源名称时匹配。对应 Java `filter`。
    fn filter(&self, config: &SQLParserVO) -> Result<bool, ELSQLException> {
        Ok(config
            .baomidou_data_source
            .as_deref()
            .is_some_and(|name| !name.trim().is_empty()))
    }

    /// 打开同名注册数据源。对应 Java `getConn`。
    fn get_conn(&self, config: &SQLParserVO) -> Result<Connection, ELSQLException> {
        let name = config
            .baomidou_data_source
            .as_deref()
            .ok_or_else(|| ELSQLException::new("baomidouDataSource is blank"))?;
        LiteflowDataSourceConnectFactory::open_data_source(name)
    }

    fn name(&self) -> &'static str {
        "BaoMiDouDynamicDsConn"
    }
}
