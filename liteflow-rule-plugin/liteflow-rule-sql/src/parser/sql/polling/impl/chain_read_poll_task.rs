//! Chain SQL 轮询任务。

use std::sync::Arc;

use liteflow_core::builder::el::LiteFlowChainELBuilder;
use liteflow_core::flow::flow_bus::FlowBus;

use super::super::{AbstractSqlReadPollTask, SqlReadPollTask};
use crate::parser::{
    constant::ReadType,
    sql::{
        exception::ELSQLException,
        read::{SqlRead, impls::ChainRead, vo::ChainVO},
    },
};

/// 对账 Chain 表并原子新增、更新或删除 FlowBus 中的 Chain。
///
/// 对应 Java:
/// `com.yomahub.liteflow.parser.sql.polling.impl.ChainReadPollTask`。
pub struct ChainReadPollTask {
    read: Arc<ChainRead>,
    bus: FlowBus,
    snapshot: AbstractSqlReadPollTask,
}

impl ChainReadPollTask {
    /// 创建 Chain 轮询任务，并校验读取类型。对应 Java 构造器。
    #[must_use]
    pub fn new(read: Arc<ChainRead>, bus: FlowBus) -> Self {
        Self {
            read,
            bus,
            snapshot: AbstractSqlReadPollTask::new(),
        }
    }

    /// 保存新增或变化的 Chain。
    ///
    /// 参数 `save_elements` 是本轮 SQL 对账识别出的新增或变化记录；每条记录都按
    /// Java 调用顺序设置 chainId、route、namespace 与 body，再通过标准 Builder
    /// 原子替换 FlowBus 中的 Chain。构建失败时返回包含原始原因的 SQL EL 异常。
    /// 对应 Java: `ChainReadPollTask#doSave`。
    pub fn do_save(&self, save_elements: &[ChainVO]) -> Result<(), ELSQLException> {
        for chain in save_elements {
            let builder = LiteFlowChainELBuilder::create_chain(self.bus.clone());
            builder.set_chain_id(&chain.chain_id);
            builder.set_route(chain.route.as_deref().unwrap_or_default());
            builder.set_namespace(chain.namespace.as_deref().unwrap_or_default());
            builder
                .set_el(&chain.body)
                .and_then(|_| builder.build())
                .map_err(|error| ELSQLException::new(error.to_string()))?;
        }
        Ok(())
    }

    /// 删除数据库中已不存在的 Chain。对应 Java `doDelete`。
    pub fn do_delete(&self, delete_ids: &[String]) {
        for chain_id in delete_ids {
            self.bus.remove_chain(chain_id);
        }
    }
}

impl SqlReadPollTask<ChainVO> for ChainReadPollTask {
    fn execute(&self) -> Result<(), ELSQLException> {
        let data = self.read.read()?;
        let (save_elements, delete_ids) = self.snapshot.diff(
            &data,
            |chain| chain.chain_id.clone(),
            |chain| chain.body.clone(),
            |chain| chain.route.clone(),
        );
        self.do_save(&save_elements)?;
        self.do_delete(&delete_ids);
        Ok(())
    }

    fn init_data(&self, data_list: &[ChainVO]) {
        self.snapshot.init_data(
            data_list,
            |chain| chain.chain_id.clone(),
            |chain| chain.body.clone(),
            |chain| chain.route.clone(),
        );
    }

    fn read_type(&self) -> ReadType {
        ReadType::Chain
    }
}
