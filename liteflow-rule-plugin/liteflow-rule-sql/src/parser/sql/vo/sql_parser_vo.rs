//! SQL 规则源扩展配置。

use serde::{Deserialize, Serialize};

/// 保存 SQL 连接、字段映射、轮询与 instanceId 配置。
///
/// serde 采用 Java/Jackson 的 camelCase 字段名，同时 Rust 字段和方法保持
/// snake_case。对应 Java: `com.yomahub.liteflow.parser.sql.vo.SQLParserVO`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SQLParserVO {
    /// JDBC/SQLite 连接地址；Rust 支持裸路径、`jdbc:sqlite:` 和 `sqlite://`。
    pub url: Option<String>,
    /// Java 驱动类名；Rust 保留为配置诊断信息。
    pub driver_class_name: Option<String>,
    /// 数据库账号名。
    pub username: Option<String>,
    /// 数据库密码。
    pub password: Option<String>,
    /// 应用名。
    pub application_name: Option<String>,
    /// Chain 表名。
    pub chain_table_name: Option<String>,
    /// Chain 表应用名字段。
    pub chain_application_name_field: String,
    /// Chain id 字段。
    pub chain_name_field: String,
    /// Chain EL 字段。
    pub el_data_field: String,
    /// instanceId 表名。
    pub instance_id_table_name: String,
    /// instanceId 应用名字段。
    pub instance_id_application_name_field: String,
    /// instanceId Chain id 字段。
    pub instance_chain_id_field: String,
    /// EL 摘要字段。
    pub el_data_md5_field: String,
    /// 节点实例编号 JSON 字段。
    pub node_instance_id_map_json_field: String,
    /// 决策路由字段。
    pub route_field: Option<String>,
    /// 命名空间字段。
    pub namespace_field: Option<String>,
    /// Chain 启用字段。
    pub chain_enable_field: Option<String>,
    /// Chain 自定义 SQL。
    pub chain_custom_sql: Option<String>,
    /// 脚本自定义 SQL。
    pub script_custom_sql: Option<String>,
    /// 脚本表名。
    pub script_table_name: Option<String>,
    /// 脚本表应用名字段。
    pub script_application_name_field: String,
    /// 脚本 id 字段。
    pub script_id_field: String,
    /// 脚本名称字段。
    pub script_name_field: String,
    /// 脚本文本字段。
    pub script_data_field: String,
    /// 脚本类型字段。
    pub script_type_field: String,
    /// 脚本语言字段。
    pub script_language_field: Option<String>,
    /// 脚本启用字段。
    pub script_enable_field: Option<String>,
    /// 是否开启轮询。
    pub polling_enabled: bool,
    /// 轮询间隔秒数。
    pub polling_interval_seconds: u64,
    /// 首次轮询延迟秒数。
    pub polling_start_seconds: u64,
    /// 是否记录 SQL。
    pub sql_log_enabled: bool,
    /// 苞米豆动态数据源名称。
    pub baomidou_data_source: Option<String>,
    /// Sharding JDBC 数据源名称。
    pub sharding_jdbc_data_source: Option<String>,
}

impl Default for SQLParserVO {
    fn default() -> Self {
        Self {
            url: None,
            driver_class_name: None,
            username: None,
            password: None,
            application_name: None,
            chain_table_name: None,
            chain_application_name_field: "application_name".to_string(),
            chain_name_field: "chain_name".to_string(),
            el_data_field: "el_data".to_string(),
            instance_id_table_name: "node_instance_id_table".to_string(),
            instance_id_application_name_field: "application_name".to_string(),
            instance_chain_id_field: "chain_id".to_string(),
            el_data_md5_field: "el_data_md5".to_string(),
            node_instance_id_map_json_field: "node_instance_id_map_json".to_string(),
            route_field: None,
            namespace_field: None,
            chain_enable_field: None,
            chain_custom_sql: None,
            script_custom_sql: None,
            script_table_name: None,
            script_application_name_field: "application_name".to_string(),
            script_id_field: "script_id".to_string(),
            script_name_field: "script_name".to_string(),
            script_data_field: "script_data".to_string(),
            script_type_field: "script_type".to_string(),
            script_language_field: None,
            script_enable_field: None,
            polling_enabled: false,
            polling_interval_seconds: 60,
            polling_start_seconds: 60,
            sql_log_enabled: true,
            baomidou_data_source: None,
            sharding_jdbc_data_source: None,
        }
    }
}

impl SQLParserVO {
    /// 使用 SQLite 路径创建默认字段映射配置。
    ///
    /// 为兼容早期 Rust `SqlRuleSource`，默认表/字段使用 `chain_id`、`el_data`。
    #[must_use]
    pub fn sqlite(db_path: impl Into<String>) -> Self {
        Self {
            url: Some(db_path.into()),
            driver_class_name: Some("org.sqlite.JDBC".to_string()),
            username: Some(String::new()),
            password: Some(String::new()),
            chain_table_name: Some("chain".to_string()),
            chain_name_field: "chain_id".to_string(),
            el_data_field: "el_data".to_string(),
            script_table_name: Some("script".to_string()),
            script_id_field: "node_id".to_string(),
            script_name_field: "name".to_string(),
            script_data_field: "script".to_string(),
            script_type_field: "script_type".to_string(),
            script_language_field: Some("language".to_string()),
            chain_enable_field: Some("enable".to_string()),
            namespace_field: Some("namespace".to_string()),
            route_field: Some("route".to_string()),
            chain_custom_sql: Some("SELECT * FROM chain".to_string()),
            script_custom_sql: Some("SELECT * FROM script".to_string()),
            ..Self::default()
        }
    }

    /// 判断是否应自动从已注册数据源中查找。对应 Java `isAutoFoundDataSource`。
    #[must_use]
    pub fn is_auto_found_data_source(&self) -> bool {
        is_blank(self.url.as_deref())
            && is_blank(self.username.as_deref())
            && is_blank(self.password.as_deref())
            && is_blank(self.driver_class_name.as_deref())
    }

    /// 判断是否使用显式连接配置。对应 Java `isUseJdbcConn`。
    ///
    /// SQLite 允许空账号密码，因此 Rust 以存在 URL 和驱动名作为显式连接依据。
    #[must_use]
    pub fn is_use_jdbc_conn(&self) -> bool {
        !is_blank(self.url.as_deref()) && !is_blank(self.driver_class_name.as_deref())
    }

    /// 判断 Chain 或脚本是否配置启用字段。对应 Java `hasEnableField`。
    #[must_use]
    pub fn has_enable_field(&self) -> bool {
        !is_blank(self.chain_enable_field.as_deref())
            || !is_blank(self.script_enable_field.as_deref())
    }
}

fn is_blank(value: Option<&str>) -> bool {
    value.is_none_or(|value| value.trim().is_empty())
}
