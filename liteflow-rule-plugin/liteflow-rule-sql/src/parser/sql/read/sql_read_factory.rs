//! SQL 读取器工厂。

use std::sync::Arc;

use super::impls::{ChainRead, InstanceIdRead, ScriptRead};
use crate::parser::sql::vo::SQLParserVO;

/// 按同一份 SQL 配置创建并共享三类读取器。
///
/// Rust 使用显式工厂实例替代 Java 静态可变 `READ_MAP`，避免不同 LiteFlow
/// 运行时之间互相覆盖。对应 Java:
/// `com.yomahub.liteflow.parser.sql.read.SqlReadFactory`。
#[derive(Debug, Clone)]
pub struct SqlReadFactory {
    chain_read: Arc<ChainRead>,
    script_read: Arc<ScriptRead>,
    instance_id_read: Arc<InstanceIdRead>,
}

impl SqlReadFactory {
    /// 注册并构造三类读取器。对应 Java `SqlReadFactory#registerRead`。
    #[must_use]
    pub fn register_read(config: SQLParserVO) -> Self {
        Self {
            chain_read: Arc::new(ChainRead::new(config.clone())),
            script_read: Arc::new(ScriptRead::new(config.clone())),
            instance_id_read: Arc::new(InstanceIdRead::new(config)),
        }
    }

    /// 返回 Chain 读取器。
    #[must_use]
    pub fn chain_read(&self) -> Arc<ChainRead> {
        self.chain_read.clone()
    }

    /// 返回脚本读取器。
    #[must_use]
    pub fn script_read(&self) -> Arc<ScriptRead> {
        self.script_read.clone()
    }

    /// 返回节点实例编号读取器。
    #[must_use]
    pub fn instance_id_read(&self) -> Arc<InstanceIdRead> {
        self.instance_id_read.clone()
    }
}
