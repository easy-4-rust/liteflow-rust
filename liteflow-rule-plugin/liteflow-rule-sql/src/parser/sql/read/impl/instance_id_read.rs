//! 节点实例编号 SQL 读取器。

use rusqlite::Row;

use super::super::abstract_sql_read::{quote, required, required_string};
use super::super::vo::InstanceIdVO;
use super::super::{AbstractSqlRead, SqlRead};
use crate::parser::{
    constant::ReadType,
    sql::{exception::ELSQLException, vo::SQLParserVO},
};

/// 读取节点实例编号持久化表。
///
/// 对应 Java: `com.yomahub.liteflow.parser.sql.read.impl.InstanceIdRead`。
#[derive(Debug, Clone)]
pub struct InstanceIdRead {
    config: SQLParserVO,
}

impl InstanceIdRead {
    /// 使用 SQL 配置创建读取器。对应 Java `InstanceIdRead#InstanceIdRead`。
    #[must_use]
    pub fn new(config: SQLParserVO) -> Self {
        Self { config }
    }
}

impl AbstractSqlRead<InstanceIdVO> for InstanceIdRead {
    fn config(&self) -> &SQLParserVO {
        &self.config
    }

    /// 映射 instanceId 查询行。对应 Java `InstanceIdRead#parse`。
    fn parse_row(&self, row: &Row<'_>) -> Result<InstanceIdVO, ELSQLException> {
        Ok(InstanceIdVO {
            chain_id: self
                .get_string_from_row_with_check(row, &self.config.instance_chain_id_field)?,
            el_data_md5: self
                .get_string_from_row_with_check(row, &self.config.el_data_md5_field)?,
            node_instance_id_map_json: self.get_string_from_row_with_check(
                row,
                &self.config.node_instance_id_map_json_field,
            )?,
        })
    }

    fn has_enable_field(&self) -> bool {
        true
    }

    fn get_enable_field_value(&self, _row: &Row<'_>) -> Result<bool, ELSQLException> {
        Ok(true)
    }

    /// 构造实例编号查询 SQL。对应 Java `InstanceIdRead#buildQuerySql`。
    fn build_query_sql(&self, object_id: Option<&str>) -> Result<String, ELSQLException> {
        let application_name =
            required(self.config.application_name.as_deref(), "applicationName")?;
        let base = format!(
            "SELECT * FROM {} WHERE {}='{}'",
            self.config.instance_id_table_name,
            self.config.instance_id_application_name_field,
            quote(application_name)
        );
        Ok(match object_id {
            Some(chain_id) if !chain_id.trim().is_empty() => format!(
                "{base} AND {}='{}'",
                self.config.instance_chain_id_field,
                quote(chain_id)
            ),
            _ => base,
        })
    }

    fn check_config(&self) -> Result<(), ELSQLException> {
        required_string(&self.config.instance_id_table_name, "tableName")?;
        required_string(&self.config.instance_chain_id_field, "chainNameField")?;
        Ok(())
    }
}

impl SqlRead<InstanceIdVO> for InstanceIdRead {
    fn read(&self) -> Result<Vec<InstanceIdVO>, ELSQLException> {
        self.read_rows(None)
    }

    fn read_by_id(&self, chain_id: &str) -> Result<Vec<InstanceIdVO>, ELSQLException> {
        if chain_id.trim().is_empty() {
            return Err(ELSQLException::new("You did not define the chainId"));
        }
        self.read_rows(Some(chain_id))
    }

    fn read_type(&self) -> ReadType {
        ReadType::InstanceId
    }
}
