//! Chain SQL 读取器。

use rusqlite::Row;

use super::super::abstract_sql_read::{quote, required, required_string};
use super::super::vo::ChainVO;
use super::super::{AbstractSqlRead, SqlRead};
use crate::parser::{
    constant::ReadType,
    sql::{exception::ELSQLException, vo::SQLParserVO},
};

/// 读取 Chain 表并映射 id、EL、namespace 与 route。
///
/// 对应 Java: `com.yomahub.liteflow.parser.sql.read.impl.ChainRead`。
#[derive(Debug, Clone)]
pub struct ChainRead {
    config: SQLParserVO,
}

impl ChainRead {
    /// 使用 SQL 配置创建读取器。对应 Java `ChainRead#ChainRead`。
    #[must_use]
    pub fn new(config: SQLParserVO) -> Self {
        Self { config }
    }
}

impl AbstractSqlRead<ChainVO> for ChainRead {
    fn config(&self) -> &SQLParserVO {
        &self.config
    }

    /// 映射 Chain 查询行。对应 Java `ChainRead#parse`。
    fn parse_row(&self, row: &Row<'_>) -> Result<ChainVO, ELSQLException> {
        let namespace = match self.config.namespace_field.as_deref() {
            Some(field) if !field.trim().is_empty() => self.get_string_from_row(row, field)?,
            _ => None,
        };
        let route = match self.config.route_field.as_deref() {
            Some(field) if !field.trim().is_empty() => self.get_string_from_row(row, field)?,
            _ => None,
        };
        Ok(ChainVO {
            chain_id: self.get_string_from_row_with_check(row, &self.config.chain_name_field)?,
            route,
            namespace,
            body: self.get_string_from_row_with_check(row, &self.config.el_data_field)?,
        })
    }

    fn has_enable_field(&self) -> bool {
        self.config
            .chain_enable_field
            .as_deref()
            .is_some_and(|field| !field.trim().is_empty())
    }

    fn get_enable_field_value(&self, row: &Row<'_>) -> Result<bool, ELSQLException> {
        let field = required(
            self.config.chain_enable_field.as_deref(),
            "chainEnableField",
        )?;
        Ok(row.get::<_, i64>(field)? == 1)
    }

    /// 构造 Chain 查询 SQL。对应 Java `ChainRead#buildQuerySql`。
    fn build_query_sql(&self, object_id: Option<&str>) -> Result<String, ELSQLException> {
        if let Some(custom_sql) = self
            .config
            .chain_custom_sql
            .as_deref()
            .filter(|sql| !sql.trim().is_empty())
        {
            return Ok(custom_sql.to_string());
        }
        let table = required(self.config.chain_table_name.as_deref(), "chainTableName")?;
        let application_name =
            required(self.config.application_name.as_deref(), "applicationName")?;
        let base = format!(
            "SELECT * FROM {table} WHERE {}='{}'",
            self.config.chain_application_name_field,
            quote(application_name)
        );
        Ok(match object_id {
            Some(chain_id) if !chain_id.trim().is_empty() => format!(
                "{base} AND {}='{}'",
                self.config.chain_name_field,
                quote(chain_id)
            ),
            _ => base,
        })
    }

    fn check_config(&self) -> Result<(), ELSQLException> {
        required(self.config.chain_table_name.as_deref(), "chainTableName")?;
        required_string(&self.config.el_data_field, "elDataField")?;
        required_string(&self.config.chain_name_field, "chainNameField")?;
        if self.config.chain_custom_sql.is_none() {
            required_string(
                &self.config.chain_application_name_field,
                "chainApplicationNameField",
            )?;
            required(self.config.application_name.as_deref(), "applicationName")?;
        }
        Ok(())
    }
}

impl SqlRead<ChainVO> for ChainRead {
    fn read(&self) -> Result<Vec<ChainVO>, ELSQLException> {
        self.read_rows(None)
    }

    fn read_by_id(&self, chain_id: &str) -> Result<Vec<ChainVO>, ELSQLException> {
        if chain_id.trim().is_empty() {
            return Err(ELSQLException::new("You did not define the chainId"));
        }
        self.read_rows(Some(chain_id))
    }

    fn read_type(&self) -> ReadType {
        ReadType::Chain
    }
}
