use serde::{Deserialize, Serialize};

/// MySQL 记忆后端配置。
///
/// 数据源由宿主容器预先注册，LiteFlow 不创建连接池。
///
/// 对应 Java: `com.yomahub.liteflow.property.agent.MysqlMemoryConfig`。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MysqlMemoryConfig {
    /// 宿主中 `DataSource` 对应的 Bean 名称。
    pub data_source_bean_name: Option<String>,
    /// 可选数据库名；缺省时由 AgentScope 使用 `agentscope`。
    pub database_name: Option<String>,
    /// 可选表名；缺省时由 AgentScope 使用 `agentscope_sessions`。
    pub table_name: Option<String>,
    /// 是否允许 AgentScope 自动建库建表，默认关闭。
    pub create_if_not_exist: bool,
}

impl MysqlMemoryConfig {
    /// 返回数据源 Bean 名称。对应 Java: `MysqlMemoryConfig#getDataSourceBeanName`。
    #[must_use]
    pub fn data_source_bean_name(&self) -> Option<&str> {
        self.data_source_bean_name.as_deref()
    }

    /// 设置数据源 Bean 名称。对应 Java: `MysqlMemoryConfig#setDataSourceBeanName`。
    pub fn set_data_source_bean_name(&mut self, data_source_bean_name: Option<String>) {
        self.data_source_bean_name = data_source_bean_name;
    }

    /// 返回数据库名。对应 Java: `MysqlMemoryConfig#getDatabaseName`。
    #[must_use]
    pub fn database_name(&self) -> Option<&str> {
        self.database_name.as_deref()
    }

    /// 设置数据库名。对应 Java: `MysqlMemoryConfig#setDatabaseName`。
    pub fn set_database_name(&mut self, database_name: Option<String>) {
        self.database_name = database_name;
    }

    /// 返回表名。对应 Java: `MysqlMemoryConfig#getTableName`。
    #[must_use]
    pub fn table_name(&self) -> Option<&str> {
        self.table_name.as_deref()
    }

    /// 设置表名。对应 Java: `MysqlMemoryConfig#setTableName`。
    pub fn set_table_name(&mut self, table_name: Option<String>) {
        self.table_name = table_name;
    }

    /// 返回是否允许自动建库建表。对应 Java: `MysqlMemoryConfig#isCreateIfNotExist`。
    #[must_use]
    pub fn is_create_if_not_exist(&self) -> bool {
        self.create_if_not_exist
    }

    /// 设置自动建库建表开关。对应 Java: `MysqlMemoryConfig#setCreateIfNotExist`。
    pub fn set_create_if_not_exist(&mut self, create_if_not_exist: bool) {
        self.create_if_not_exist = create_if_not_exist;
    }
}
