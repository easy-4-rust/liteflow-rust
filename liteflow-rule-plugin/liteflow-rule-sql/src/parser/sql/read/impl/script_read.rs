//! 脚本 SQL 读取器。

use rusqlite::Row;

use super::super::abstract_sql_read::{quote, required, required_string};
use super::super::vo::ScriptVO;
use super::super::{AbstractSqlRead, SqlRead};
use crate::parser::{
    constant::ReadType,
    sql::{exception::ELSQLException, util::LiteFlowJdbcUtil, vo::SQLParserVO},
};

/// 读取脚本表并映射脚本节点元数据。
///
/// 对应 Java: `com.yomahub.liteflow.parser.sql.read.impl.ScriptRead`。
#[derive(Debug, Clone)]
pub struct ScriptRead {
    config: SQLParserVO,
}

impl ScriptRead {
    /// 使用 SQL 配置创建读取器。对应 Java `ScriptRead#ScriptRead`。
    #[must_use]
    pub fn new(config: SQLParserVO) -> Self {
        Self { config }
    }
}

impl AbstractSqlRead<ScriptVO> for ScriptRead {
    fn config(&self) -> &SQLParserVO {
        &self.config
    }

    /// 映射脚本查询行。对应 Java `ScriptRead#parse`。
    fn parse_row(&self, row: &Row<'_>) -> Result<ScriptVO, ELSQLException> {
        let language = match self.config.script_language_field.as_deref() {
            Some(field) if !field.trim().is_empty() => self.get_string_from_row(row, field)?,
            _ => None,
        };
        Ok(ScriptVO {
            node_id: self.get_string_from_row_with_check(row, &self.config.script_id_field)?,
            script_type: self
                .get_string_from_row_with_check(row, &self.config.script_type_field)?,
            name: self.get_string_from_row(row, &self.config.script_name_field)?,
            language,
            enable: None,
            script: self.get_string_from_row_with_check(row, &self.config.script_data_field)?,
        })
    }

    fn has_enable_field(&self) -> bool {
        self.config
            .script_enable_field
            .as_deref()
            .is_some_and(|field| !field.trim().is_empty())
    }

    fn get_enable_field_value(&self, row: &Row<'_>) -> Result<bool, ELSQLException> {
        let field = required(
            self.config.script_enable_field.as_deref(),
            "scriptEnableField",
        )?;
        Ok(row.get::<_, i64>(field)? == 1)
    }

    /// 构造脚本查询 SQL。对应 Java `ScriptRead#buildQuerySql`。
    fn build_query_sql(&self, object_id: Option<&str>) -> Result<String, ELSQLException> {
        if let Some(custom_sql) = self
            .config
            .script_custom_sql
            .as_deref()
            .filter(|sql| !sql.trim().is_empty())
        {
            return Ok(custom_sql.to_string());
        }
        let table = required(self.config.script_table_name.as_deref(), "scriptTableName")?;
        let application_name =
            required(self.config.application_name.as_deref(), "applicationName")?;
        let base = format!(
            "SELECT * FROM {table} WHERE {}='{}'",
            self.config.script_application_name_field,
            quote(application_name)
        );
        Ok(match object_id {
            Some(script_id) if !script_id.trim().is_empty() => format!(
                "{base} AND {}='{}'",
                self.config.script_id_field,
                quote(script_id)
            ),
            _ => base,
        })
    }

    fn check_config(&self) -> Result<(), ELSQLException> {
        required(self.config.script_table_name.as_deref(), "scriptTableName")?;
        required_string(&self.config.script_id_field, "scriptIdField")?;
        required_string(&self.config.script_data_field, "scriptDataField")?;
        required_string(&self.config.script_type_field, "scriptTypeField")?;
        if self.config.script_custom_sql.is_none() {
            required_string(
                &self.config.script_application_name_field,
                "scriptApplicationNameField",
            )?;
        }
        Ok(())
    }

    /// 脚本表未配置或不存在时跳过读取。对应 Java `ScriptRead#needRead`。
    fn need_read(&self) -> Result<bool, ELSQLException> {
        let Some(table) = self
            .config
            .script_table_name
            .as_deref()
            .filter(|table| !table.trim().is_empty())
        else {
            return Ok(false);
        };
        let connection = LiteFlowJdbcUtil::get_conn(&self.config)?;
        Ok(LiteFlowJdbcUtil::check_connection_can_execute_sql(
            &connection,
            &format!("SELECT 1 FROM {table} LIMIT 1"),
        ))
    }
}

impl SqlRead<ScriptVO> for ScriptRead {
    fn read(&self) -> Result<Vec<ScriptVO>, ELSQLException> {
        self.read_rows(None)
    }

    fn read_by_id(&self, script_id: &str) -> Result<Vec<ScriptVO>, ELSQLException> {
        if script_id.trim().is_empty() {
            return Err(ELSQLException::new("You did not define the scriptNodeId"));
        }
        self.read_rows(Some(script_id))
    }

    fn read_type(&self) -> ReadType {
        ReadType::Script
    }
}
