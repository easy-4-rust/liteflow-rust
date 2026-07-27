//! 脚本 SQL 轮询任务。

use std::sync::Arc;

use liteflow_core::builder::LiteFlowNodeBuilder;
use liteflow_core::enums::NodeTypeEnum;
use liteflow_core::flow::flow_bus::FlowBus;

use super::super::{AbstractSqlReadPollTask, SqlReadPollTask};
use crate::parser::{
    constant::ReadType,
    sql::{
        exception::ELSQLException,
        read::{SqlRead, impls::ScriptRead, vo::ScriptVO},
    },
};

/// 对账脚本表并新增、热更新或卸载脚本节点。
///
/// 对应 Java:
/// `com.yomahub.liteflow.parser.sql.polling.impl.ScriptReadPollTask`。
pub struct ScriptReadPollTask {
    read: Arc<ScriptRead>,
    bus: FlowBus,
    snapshot: AbstractSqlReadPollTask,
}

impl ScriptReadPollTask {
    /// 创建脚本轮询任务。对应 Java 构造器。
    #[must_use]
    pub fn new(read: Arc<ScriptRead>, bus: FlowBus) -> Self {
        Self {
            read,
            bus,
            snapshot: AbstractSqlReadPollTask::new(),
        }
    }

    /// 保存新增或变化的脚本节点。对应 Java `doSave`。
    pub fn do_save(&self, save_elements: &[ScriptVO]) -> Result<(), ELSQLException> {
        for script in save_elements {
            let node_type =
                NodeTypeEnum::get_enum_by_code(&script.script_type).ok_or_else(|| {
                    ELSQLException::new(format!("type [{}] is not support", script.script_type))
                })?;
            let mut builder = LiteFlowNodeBuilder::create_script_node(&self.bus)
                .set_id(&script.node_id)
                .set_type(node_type)
                .set_script(&script.script);
            if let Some(name) = &script.name {
                builder = builder.set_name(name);
            }
            if let Some(language) = &script.language {
                builder = builder.set_language(language);
            }
            builder
                .build()
                .map_err(|error| ELSQLException::new(error.to_string()))?;
        }
        Ok(())
    }

    /// 卸载数据库中已不存在的脚本节点。对应 Java `doDelete`。
    pub fn do_delete(&self, delete_ids: &[String]) {
        for node_id in delete_ids {
            self.bus.unregister(node_id);
        }
    }
}

impl SqlReadPollTask<ScriptVO> for ScriptReadPollTask {
    fn execute(&self) -> Result<(), ELSQLException> {
        let data = self.read.read()?;
        let (save_elements, delete_ids) = self.snapshot.diff(
            &data,
            |script| script.node_id.clone(),
            |script| script.script.clone(),
            |_| None,
        );
        self.do_save(&save_elements)?;
        self.do_delete(&delete_ids);
        Ok(())
    }

    fn init_data(&self, data_list: &[ScriptVO]) {
        self.snapshot.init_data(
            data_list,
            |script| script.node_id.clone(),
            |script| script.script.clone(),
            |_| None,
        );
    }

    fn read_type(&self) -> ReadType {
        ReadType::Script
    }
}
