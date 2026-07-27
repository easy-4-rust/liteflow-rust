//! SQL 读取和 XML 组装常量。

/// 保存 Java SQL 插件使用的固定 SQL/XML 模板。
///
/// Rust 查询优先使用参数绑定，但保留这些模板用于配置对照与诊断。
/// 对应 Java: `com.yomahub.liteflow.parser.constant.SqlReadConstant`。
pub struct SqlReadConstant;

impl SqlReadConstant {
    /// instanceId 存在性查询模板。
    pub const INSTANT_SELECT_SQL: &'static str =
        "SELECT count(*) FROM {} where {} = '{}' and {} = '{}'";
    /// instanceId 更新模板。
    pub const INSTANT_UPDATE_SQL: &'static str =
        "UPDATE {} SET {} = '{}',{} = '{}' WHERE {} = '{}' and {} = '{}'";
    /// instanceId 插入模板。
    pub const INSTANT_INSERT_SQL: &'static str =
        "INSERT INTO {} ({},{},{},{}) VALUES ('{}','{}','{}','{}')";
    /// 默认节点实例编号表建表语句。
    pub const INSTANT_CREATE_TABLE_SQL: &'static str =
        "CREATE TABLE IF NOT EXISTS node_instance_id_table (
application_name TEXT NOT NULL,
chain_id TEXT NOT NULL,
el_data_md5 TEXT NOT NULL,
node_instance_id_map_json TEXT NOT NULL,
PRIMARY KEY(application_name, chain_id)
)";
    /// 按应用名读取数据的默认模板。
    pub const SQL_PATTERN: &'static str = "SELECT * FROM {} WHERE {}='{}'";
    /// 按应用名和对象 id 读取数据的默认模板。
    pub const SQL_PATTERN_WITH_CHAIN_ID: &'static str =
        "SELECT * FROM {} WHERE {}='{}' and {}='{}'";
    /// 检查脚本表是否存在且可查询。
    pub const SCRIPT_SQL_CHECK_PATTERN: &'static str = "SELECT 1 FROM {}";
    /// 脚本查询模板。
    pub const SCRIPT_SQL_PATTERN: &'static str = "SELECT * FROM {} WHERE {}='{}'";
    /// 单条 Chain XML 模板。
    pub const CHAIN_XML_PATTERN: &'static str = "<chain id=\"{}\" namespace=\"{}\"><route><![CDATA[{}]]></route><body><![CDATA[{}]]></body></chain>";
    /// 脚本节点集合 XML 模板。
    pub const NODE_XML_PATTERN: &'static str = "<nodes>{}</nodes>";
    /// 未指定脚本语言的节点 XML 模板。
    pub const NODE_ITEM_XML_PATTERN: &'static str =
        "<node id=\"{}\" name=\"{}\" type=\"{}\"><![CDATA[{}]]></node>";
    /// 指定脚本语言的节点 XML 模板。
    pub const NODE_ITEM_WITH_LANGUAGE_XML_PATTERN: &'static str =
        "<node id=\"{}\" name=\"{}\" type=\"{}\" language=\"{}\"><![CDATA[{}]]></node>";
    /// 完整规则 XML 模板。
    pub const XML_PATTERN: &'static str =
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><flow>{}{}</flow>";
    /// JDBC 最大游标拉取数量；rusqlite 逐行迭代天然保持流式读取。
    pub const FETCH_SIZE_MAX: usize = 1_000;
}
