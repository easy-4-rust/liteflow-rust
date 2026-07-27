//! Chain SQL 轮询任务。

use std::sync::Arc;

use liteflow_core::builder::el::LiteFlowChainELBuilder;
use liteflow_core::el::parse_el;
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

    /// 保存新增或变化的 Chain。对应 Java `doSave`。
    pub fn do_save(&self, save_elements: &[ChainVO]) -> Result<(), ELSQLException> {
        for chain in save_elements {
            if let Some(route) = chain
                .route
                .as_deref()
                .filter(|route| !route.trim().is_empty())
            {
                let route_el =
                    parse_el(route).map_err(|error| ELSQLException::new(error.to_string()))?;
                let body_el = parse_el(&chain.body)
                    .map_err(|error| ELSQLException::new(error.to_string()))?;
                let namespace = chain.namespace.as_deref().unwrap_or("DEFAULT");
                let built = LiteFlowChainELBuilder::new(self.bus.clone())
                    .build_route_chain(&chain.chain_id, namespace, route_el, body_el)
                    .map_err(|error| ELSQLException::new(error.to_string()))?;
                self.bus.add_built_chain(built);
            } else {
                self.bus
                    .add_chain(&chain.chain_id, &chain.body)
                    .map_err(|error| ELSQLException::new(error.to_string()))?;
            }
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
