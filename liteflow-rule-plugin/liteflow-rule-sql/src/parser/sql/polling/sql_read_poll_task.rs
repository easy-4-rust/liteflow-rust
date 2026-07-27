//! SQL 轮询任务契约。

use crate::parser::{constant::ReadType, sql::exception::ELSQLException};

/// 对账 SQL 快照并把增删改应用到运行中的 FlowBus。
///
/// 对应 Java: `com.yomahub.liteflow.parser.sql.polling.SqlReadPollTask`。
pub trait SqlReadPollTask<T>: Send + Sync {
    /// 执行一次轮询。对应 Java `execute()`。
    fn execute(&self) -> Result<(), ELSQLException>;

    /// 使用首次全量读取结果初始化摘要快照。对应 Java `initData(List)`。
    fn init_data(&self, data_list: &[T]);

    /// 返回轮询对象类型。对应 Java `type()`。
    fn read_type(&self) -> ReadType;
}
