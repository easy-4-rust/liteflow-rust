//! 对应 Java: `com.yomahub.liteflow.parser.sql.SqlParser`。
//!
//! 表结构采用 Rust 默认约定：
//! - chain：chain_id / namespace / el_data / route / body / enable
//! - script：node_id / script_type / language / script

use async_trait::async_trait;
use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::rule_plugin::{RuleFormat, RuleSource, fnv_fp};

/// SQLite 规则源。
///
/// Java SQL 插件支持 JDBC 多数据源与字段映射；Rust 默认实现使用 rusqlite，
/// 拉取后组装标准 JSON 规则文本，复用 core 的 serde parser。
pub struct SqlRuleSource {
    pub db_path: String,
    pub chain_table: String,
    pub script_table: String,
}

impl SqlRuleSource {
    /// 创建 SQL 规则源。
    pub fn new(db_path: impl Into<String>) -> Self {
        Self {
            db_path: db_path.into(),
            ..Self::default()
        }
    }
}

impl Default for SqlRuleSource {
    fn default() -> Self {
        Self {
            db_path: String::new(),
            chain_table: "chain".to_string(),
            script_table: "script".to_string(),
        }
    }
}

#[async_trait]
impl RuleSource for SqlRuleSource {
    /// 查询 chain/script 表并生成 JSON 规则。对应 Java `SqlParser#parseCustom`。
    async fn fetch(&self) -> LFResult<(String, String)> {
        let connection = rusqlite::Connection::open(&self.db_path)
            .map_err(|error| LiteflowError::Rule(format!("sql open error: {error}")))?;

        let mut chains = Vec::new();
        {
            let mut statement = connection
                .prepare(&format!(
                    "SELECT chain_id, COALESCE(namespace,'DEFAULT'), el_data, route, body, enable FROM {}",
                    self.chain_table
                ))
                .map_err(|error| {
                    LiteflowError::Rule(format!("sql prepare error: {error}"))
                })?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<i64>>(5)?,
                    ))
                })
                .map_err(|error| LiteflowError::Rule(format!("sql query error: {error}")))?;
            for row in rows {
                let (id, namespace, el, route, body, enable) =
                    row.map_err(|error| LiteflowError::Rule(format!("sql row error: {error}")))?;
                let mut chain = serde_json::json!({"id": id, "namespace": namespace});
                if let Some(el) = el {
                    chain["body"] = serde_json::Value::String(el);
                }
                if let Some(route) = route {
                    chain["route"] = serde_json::Value::String(route);
                    if let Some(body) = body {
                        chain["body"] = serde_json::Value::String(body);
                    }
                }
                if let Some(enable) = enable {
                    chain["enable"] = serde_json::Value::Bool(enable != 0);
                }
                chains.push(chain);
            }
        }

        let mut nodes = Vec::new();
        if connection
            .prepare(&format!("SELECT 1 FROM {} LIMIT 1", self.script_table))
            .is_ok()
        {
            let mut statement = connection
                .prepare(&format!(
                    "SELECT node_id, script_type, language, script FROM {}",
                    self.script_table
                ))
                .map_err(|error| LiteflowError::Rule(format!("sql prepare error: {error}")))?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })
                .map_err(|error| LiteflowError::Rule(format!("sql query error: {error}")))?;
            for row in rows {
                let (id, script_type, language, script) =
                    row.map_err(|error| LiteflowError::Rule(format!("sql row error: {error}")))?;
                nodes.push(serde_json::json!({
                    "id": id,
                    "type": script_type,
                    "language": language,
                    "script": script
                }));
            }
        }

        let rule = serde_json::json!({
            "flow": {"chain": chains, "nodes": {"node": nodes}}
        });
        let text = serde_json::to_string(&rule)
            .map_err(|error| LiteflowError::Rule(format!("sql build error: {error}")))?;
        Ok((text.clone(), fnv_fp(&text)))
    }

    fn format(&self) -> RuleFormat {
        RuleFormat::Json
    }

    fn name(&self) -> &str {
        "sql"
    }
}
