//! 对应 liteflow-rule-sql：SQL 数据库规则源。
//! 表结构对齐 Java 插件：chain 表（chain_id/namespace/el_data/route/body/enable）、
//! script 表（node_id/script_type/language/script）。Rust 默认实现基于 rusqlite。

use super::rule_source::{fnv_fp, RuleFormat, RuleSource};
use async_trait::async_trait;
use crate::exception::{LFResult, LiteflowError};

/// SQL 规则源（对应 SqlParser；拉取后组装为 JSON 规则文本复用解析器）
pub struct SqlRuleSource {
    pub db_path: String,
    pub chain_table: String,
    pub script_table: String,
}

impl Default for SqlRuleSource {
    fn default() -> Self {
        Self {
            db_path: String::new(),
            chain_table: "chain".into(),
            script_table: "script".into(),
        }
    }
}

#[async_trait]
impl RuleSource for SqlRuleSource {
    async fn fetch(&self) -> LFResult<(String, String)> {
        let conn = rusqlite::Connection::open(&self.db_path)
            .map_err(|e| LiteflowError::Rule(format!("sql open error: {e}")))?;

        let mut chains = Vec::new();
        {
            let mut stmt = conn
                .prepare(&format!(
                    "SELECT chain_id, COALESCE(namespace,'DEFAULT'), el_data, route, body, enable FROM {}",
                    self.chain_table
                ))
                .map_err(|e| LiteflowError::Rule(format!("sql prepare error: {e}")))?;
            let rows = stmt
                .query_map([], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, Option<String>>(2)?,
                        r.get::<_, Option<String>>(3)?,
                        r.get::<_, Option<String>>(4)?,
                        r.get::<_, Option<i64>>(5)?,
                    ))
                })
                .map_err(|e| LiteflowError::Rule(format!("sql query error: {e}")))?;
            for row in rows.flatten() {
                let (id, ns, el, route, body, enable) = row;
                let mut c = serde_json::json!({"id": id, "namespace": ns});
                if let Some(e) = el {
                    c["body"] = serde_json::Value::String(e);
                }
                if let Some(r) = route {
                    c["route"] = serde_json::Value::String(r);
                    if let Some(b) = body {
                        c["body"] = serde_json::Value::String(b);
                    }
                }
                if let Some(en) = enable {
                    c["enable"] = serde_json::Value::Bool(en != 0);
                }
                chains.push(c);
            }
        }

        let mut nodes = Vec::new();
        if conn
            .prepare(&format!("SELECT 1 FROM {} LIMIT 1", self.script_table))
            .is_ok()
        {
            let mut stmt = conn
                .prepare(&format!(
                    "SELECT node_id, script_type, language, script FROM {}",
                    self.script_table
                ))
                .map_err(|e| LiteflowError::Rule(format!("sql prepare error: {e}")))?;
            let rows = stmt
                .query_map([], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, Option<String>>(2)?,
                        r.get::<_, String>(3)?,
                    ))
                })
                .map_err(|e| LiteflowError::Rule(format!("sql query error: {e}")))?;
            for row in rows.flatten() {
                let (id, st, lang, script) = row;
                nodes.push(serde_json::json!({
                    "id": id, "type": st, "language": lang, "script": script
                }));
            }
        }

        let rule = serde_json::json!({
            "flow": { "chain": chains, "nodes": { "node": nodes } }
        });
        let text = serde_json::to_string(&rule)
            .map_err(|e| LiteflowError::Rule(format!("sql build error: {e}")))?;
        let fp = fnv_fp(&text);
        Ok((text, fp))
    }
    fn format(&self) -> RuleFormat {
        RuleFormat::Json
    }
    fn name(&self) -> &str {
        "sql"
    }
}
