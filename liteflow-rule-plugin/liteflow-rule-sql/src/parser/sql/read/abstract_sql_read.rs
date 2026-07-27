//! SQL 读取公共基座。

use rusqlite::Row;

use crate::parser::sql::{exception::ELSQLException, util::LiteFlowJdbcUtil, vo::SQLParserVO};

/// 提供配置检查、连接获取、逐行映射和启停过滤公共算法。
///
/// 对应 Java: `com.yomahub.liteflow.parser.sql.read.AbstractSqlRead`。
pub trait AbstractSqlRead<T>: Send + Sync {
    /// 返回当前读取器配置。
    fn config(&self) -> &SQLParserVO;

    /// 把一行结果转换为目标 VO。对应 Java `parse(ResultSet)`。
    fn parse_row(&self, row: &Row<'_>) -> Result<T, ELSQLException>;

    /// 判断结果是否包含启停字段。对应 Java `hasEnableFiled` 原拼写。
    fn has_enable_field(&self) -> bool;

    /// 返回当前行的启停字段值。对应 Java `getEnableFiledValue`。
    fn get_enable_field_value(&self, row: &Row<'_>) -> Result<bool, ELSQLException>;

    /// 构造读取 SQL；`object_id` 缺失表示全量读取。
    fn build_query_sql(&self, object_id: Option<&str>) -> Result<String, ELSQLException>;

    /// 校验当前读取器需要的配置。对应 Java `checkConfig`。
    fn check_config(&self) -> Result<(), ELSQLException>;

    /// 判断该对象类型是否可读；脚本表不存在时返回 false。
    fn need_read(&self) -> Result<bool, ELSQLException> {
        Ok(true)
    }

    /// 执行公共读取主干。
    ///
    /// 对应 Java `AbstractSqlRead#read` 与私有 `readList`。
    fn read_rows(&self, object_id: Option<&str>) -> Result<Vec<T>, ELSQLException> {
        if !self.need_read()? {
            return Ok(Vec::new());
        }
        self.check_config()?;
        let sql = self.build_query_sql(object_id)?;
        if self.config().sql_log_enabled {
            eprintln!("[liteflow-sql] query sql: {sql}");
        }

        let connection = LiteFlowJdbcUtil::get_conn(self.config())?;
        let mut statement = connection.prepare(&sql)?;
        let mut rows = statement.query([])?;
        let mut result = Vec::new();
        while let Some(row) = rows.next()? {
            if self.has_enable_field() && !self.get_enable_field_value(row)? {
                continue;
            }
            result.push(self.parse_row(row)?);
        }
        Ok(result)
    }

    /// 读取字符串列；SQL NULL 映射为 `None`。对应 Java `getStringFromRs`。
    fn get_string_from_row(
        &self,
        row: &Row<'_>,
        field: &str,
    ) -> Result<Option<String>, ELSQLException> {
        row.get::<_, Option<String>>(field).map_err(Into::into)
    }

    /// 读取非空字符串列；空白值转换为 SQL 业务异常。
    ///
    /// 对应 Java `getStringFromRsWithCheck`。
    fn get_string_from_row_with_check(
        &self,
        row: &Row<'_>,
        field: &str,
    ) -> Result<String, ELSQLException> {
        self.get_string_from_row(row, field)?
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| ELSQLException::new(format!("field[{field}] value is empty")))
    }
}

pub(crate) fn required<'a>(
    value: Option<&'a str>,
    property: &str,
) -> Result<&'a str, ELSQLException> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ELSQLException::new(format!("You did not define the {property} property")))
}

pub(crate) fn required_string<'a>(
    value: &'a str,
    property: &str,
) -> Result<&'a str, ELSQLException> {
    if value.trim().is_empty() {
        Err(ELSQLException::new(format!(
            "You did not define the {property} property"
        )))
    } else {
        Ok(value)
    }
}

pub(crate) fn quote(value: &str) -> String {
    value.replace('\'', "''")
}
