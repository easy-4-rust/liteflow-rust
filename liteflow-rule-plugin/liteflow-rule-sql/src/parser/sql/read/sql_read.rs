//! SQL 读取契约。

use crate::parser::{constant::ReadType, sql::exception::ELSQLException};

/// 读取某类 SQL 配置对象。
///
/// 对应 Java: `com.yomahub.liteflow.parser.sql.read.SqlRead`。
pub trait SqlRead<T>: Send + Sync {
    /// 读取当前应用的全部对象。对应 Java `SqlRead#read()`。
    fn read(&self) -> Result<Vec<T>, ELSQLException>;

    /// 根据 Chain id 或脚本 id 读取对象。对应 Java `SqlRead#read(String)`。
    fn read_by_id(&self, object_id: &str) -> Result<Vec<T>, ELSQLException>;

    /// 返回读取类型。对应 Java `SqlRead#type()`。
    fn read_type(&self) -> ReadType;
}
