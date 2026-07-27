//! SQL 规则内容聚合与轮询调度。

use std::sync::Arc;
use std::time::Duration;

use liteflow_core::InstanceInfoDto;
use liteflow_core::flow::flow_bus::FlowBus;
use rusqlite::params;

use crate::parser::sql::{
    exception::ELSQLException,
    polling::{
        SqlReadPollTask,
        impls::{ChainReadPollTask, ScriptReadPollTask},
    },
    read::{SqlRead, SqlReadFactory, vo::ChainVO, vo::ScriptVO},
    vo::SQLParserVO,
};

use super::LiteFlowJdbcUtil;

/// 聚合 Chain/脚本读取、XML 生成、轮询调度和 instanceId upsert。
///
/// Rust 使用显式实例和可取消 Tokio 任务替代 Java 静态单例与
/// `ScheduledThreadPoolExecutor`。对应 Java:
/// `com.yomahub.liteflow.parser.sql.util.JDBCHelper`。
#[derive(Debug, Clone)]
pub struct JDBCHelper {
    config: SQLParserVO,
    read_factory: SqlReadFactory,
}

impl JDBCHelper {
    /// 初始化 JDBC helper 与读取工厂。对应 Java `JDBCHelper#init`。
    #[must_use]
    pub fn init(config: SQLParserVO) -> Self {
        Self {
            read_factory: SqlReadFactory::register_read(config.clone()),
            config,
        }
    }

    /// 返回 SQL 配置。对应 Java `getSqlParserVO`。
    #[must_use]
    pub fn sql_parser_vo(&self) -> &SQLParserVO {
        &self.config
    }

    /// 返回共享读取器工厂。
    #[must_use]
    pub fn read_factory(&self) -> &SqlReadFactory {
        &self.read_factory
    }

    /// 读取 Chain 与脚本并生成 LiteFlow XML。
    ///
    /// 对应 Java `JDBCHelper#getContent`。
    pub fn get_content(&self) -> Result<String, ELSQLException> {
        let chains = self.read_factory.chain_read().read()?;
        let scripts = self.read_factory.script_read().read()?;
        Ok(build_xml(&chains, &scripts))
    }

    /// 启动定时轮询任务，并返回可取消句柄。
    ///
    /// 首次等待 `polling_start_seconds`，之后按
    /// `polling_interval_seconds` 对账脚本和 Chain。对应 Java
    /// `JDBCHelper#listenSQL`。
    #[must_use]
    pub fn listen_sql(&self, bus: FlowBus) -> tokio::task::JoinHandle<()> {
        let chain_task = Arc::new(ChainReadPollTask::new(
            self.read_factory.chain_read(),
            bus.clone(),
        ));
        let script_task = Arc::new(ScriptReadPollTask::new(
            self.read_factory.script_read(),
            bus,
        ));

        // 初始 XML 装载后记录数据库快照，避免第一次轮询把全部对象误判为新增。
        if let Ok(chains) = self.read_factory.chain_read().read() {
            chain_task.init_data(&chains);
        }
        if let Ok(scripts) = self.read_factory.script_read().read() {
            script_task.init_data(&scripts);
        }
        let start = Duration::from_secs(self.config.polling_start_seconds);
        let interval = Duration::from_secs(self.config.polling_interval_seconds.max(1));
        tokio::spawn(async move {
            tokio::time::sleep(start).await;
            loop {
                if let Err(error) = script_task.execute() {
                    eprintln!("[liteflow-sql] poll script fail: {error}");
                }
                if let Err(error) = chain_task.execute() {
                    eprintln!("[liteflow-sql] poll chain fail: {error}");
                }
                tokio::time::sleep(interval).await;
            }
        })
    }

    /// 创建节点 instanceId 表，如果不存在。
    ///
    /// 对应 Java `JDBCHelper#createNodeInstanceIdTable`。
    pub fn create_node_instance_id_table(&self) -> Result<(), ELSQLException> {
        let table = identifier(&self.config.instance_id_table_name)?;
        let application = identifier(&self.config.instance_id_application_name_field)?;
        let chain = identifier(&self.config.instance_chain_id_field)?;
        let md5 = identifier(&self.config.el_data_md5_field)?;
        let json = identifier(&self.config.node_instance_id_map_json_field)?;
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {table} (
                {application} TEXT NOT NULL,
                {chain} TEXT NOT NULL,
                {md5} TEXT NOT NULL,
                {json} TEXT NOT NULL,
                PRIMARY KEY({application}, {chain})
            )"
        );
        LiteFlowJdbcUtil::get_conn(&self.config)?
            .execute_batch(&sql)
            .map_err(Into::into)
    }

    /// 按应用名和 Chain id 执行参数化 upsert。
    ///
    /// 对应 Java `JDBCHelper#executeUpsert`，但 Rust 使用绑定参数避免数据内容
    /// 破坏 SQL 语句。
    pub fn execute_upsert(
        &self,
        instance_id_list: &[InstanceInfoDto],
        el_md5: &str,
        chain_id: &str,
    ) -> Result<(), ELSQLException> {
        let table = identifier(&self.config.instance_id_table_name)?;
        let application_field = identifier(&self.config.instance_id_application_name_field)?;
        let chain_field = identifier(&self.config.instance_chain_id_field)?;
        let md5_field = identifier(&self.config.el_data_md5_field)?;
        let json_field = identifier(&self.config.node_instance_id_map_json_field)?;
        let application_name = self
            .config
            .application_name
            .as_deref()
            .ok_or_else(|| ELSQLException::new("applicationName is blank"))?;
        let json = serde_json::to_string(instance_id_list)
            .map_err(|error| ELSQLException::new(error.to_string()))?;

        let mut connection = LiteFlowJdbcUtil::get_conn(&self.config)?;
        let transaction = connection.transaction()?;
        let count: i64 = transaction.query_row(
            &format!(
                "SELECT count(*) FROM {table} WHERE {application_field}=?1 AND {chain_field}=?2"
            ),
            params![application_name, chain_id],
            |row| row.get(0),
        )?;
        if count > 0 {
            transaction.execute(
                &format!(
                    "UPDATE {table} SET {md5_field}=?1, {json_field}=?2 \
                     WHERE {application_field}=?3 AND {chain_field}=?4"
                ),
                params![el_md5, json, application_name, chain_id],
            )?;
        } else {
            transaction.execute(
                &format!(
                    "INSERT INTO {table} \
                     ({application_field},{json_field},{md5_field},{chain_field}) \
                     VALUES (?1,?2,?3,?4)"
                ),
                params![application_name, json, el_md5, chain_id],
            )?;
        }
        transaction.commit().map_err(Into::into)
    }
}

fn build_xml(chains: &[ChainVO], scripts: &[ScriptVO]) -> String {
    let chain_xml = chains
        .iter()
        .map(|chain| {
            let id = escape_attribute(&chain.chain_id);
            let namespace = escape_attribute(chain.namespace.as_deref().unwrap_or(""));
            let body = escape_cdata(&chain.body);
            match chain
                .route
                .as_deref()
                .filter(|route| !route.trim().is_empty())
            {
                Some(route) => format!(
                    "<chain id=\"{id}\" namespace=\"{namespace}\"><route><![CDATA[{}]]></route><body><![CDATA[{body}]]></body></chain>",
                    escape_cdata(route)
                ),
                // Java 解析器会忽略空 route；Rust XML Parser 看到空元素会尝试解析空 EL，
                // 因此在序列化边界省略空 route，保持最终运行语义一致。
                None => format!(
                    "<chain id=\"{id}\" namespace=\"{namespace}\"><body><![CDATA[{body}]]></body></chain>"
                ),
            }
        })
        .collect::<String>();
    let script_xml = scripts
        .iter()
        .map(|script| {
            let name = escape_attribute(script.name.as_deref().unwrap_or(""));
            let id = escape_attribute(&script.node_id);
            let script_type = escape_attribute(&script.script_type);
            let body = escape_cdata(&script.script);
            match script
                .language
                .as_deref()
                .filter(|language| !language.trim().is_empty())
            {
                Some(language) => format!(
                    "<node id=\"{id}\" name=\"{name}\" type=\"{script_type}\" language=\"{}\"><![CDATA[{body}]]></node>",
                    escape_attribute(language)
                ),
                None => format!(
                    "<node id=\"{id}\" name=\"{name}\" type=\"{script_type}\"><![CDATA[{body}]]></node>"
                ),
            }
        })
        .collect::<String>();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><flow><nodes>{script_xml}</nodes>{chain_xml}</flow>"
    )
}

fn escape_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_cdata(value: &str) -> String {
    value.replace("]]>", "]]]]><![CDATA[>")
}

fn identifier(value: &str) -> Result<&str, ELSQLException> {
    if !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        Ok(value)
    } else {
        Err(ELSQLException::new(format!(
            "invalid SQL identifier[{value}]"
        )))
    }
}
