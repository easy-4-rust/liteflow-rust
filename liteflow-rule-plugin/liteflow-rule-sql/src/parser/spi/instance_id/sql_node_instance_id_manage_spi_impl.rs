//! SQL 节点实例编号持久化 SPI。

use std::collections::HashMap;
use std::sync::Mutex;

use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::flow::entity::InstanceInfoDto;
use liteflow_core::flow::instance_id::NodeInstanceIdManageSpi;
use sha1::{Digest, Sha1};

use crate::parser::sql::{read::SqlRead, util::JDBCHelper, vo::SQLParserVO};

/// 把节点实例编号映射和 EL 摘要持久化到 SQL 表。
///
/// 对应 Java:
/// `com.yomahub.liteflow.parser.spi.instanceId.SqlNodeInstanceIdManageSpiImpl`。
pub struct SqlNodeInstanceIdManageSpiImpl {
    jdbc_helper: JDBCHelper,
    generated_ids: Mutex<HashMap<String, String>>,
}

impl SqlNodeInstanceIdManageSpiImpl {
    /// 使用 SQL 配置创建 SPI。
    #[must_use]
    pub fn new(config: SQLParserVO) -> Self {
        Self {
            jdbc_helper: JDBCHelper::init(config),
            generated_ids: Mutex::new(HashMap::new()),
        }
    }

    /// 创建 instanceId 表。
    pub fn create_table(&self) -> LFResult<()> {
        self.jdbc_helper
            .create_node_instance_id_table()
            .map_err(LiteflowError::from)
    }
}

impl NodeInstanceIdManageSpi for SqlNodeInstanceIdManageSpiImpl {
    /// 生成同一进程内稳定的节点实例编号。
    fn gen_instance_id(&self, chain_id: &str, node_id: &str, occurrence: usize) -> String {
        let key = format!("{chain_id}:{node_id}:{occurrence}");
        let mut ids = self
            .generated_ids
            .lock()
            .expect("SQL 节点实例编号缓存锁中毒");
        ids.entry(key.clone())
            .or_insert_with(|| {
                let digest = Sha1::digest(key.as_bytes());
                format!(
                    "{node_id}_{:08x}_{occurrence}",
                    u32::from_be_bytes(digest[..4].try_into().expect("SHA-1 前四字节"))
                )
            })
            .clone()
    }

    /// 读取指定 Chain 的 EL 摘要和节点实例编号 JSON。
    ///
    /// 返回两行，顺序与 Java 一致：`elDataMd5`、`nodeInstanceIdMapJson`。
    fn read_instance_id_file(&self, chain_id: &str) -> LFResult<Vec<String>> {
        let records = self
            .jdbc_helper
            .read_factory()
            .instance_id_read()
            .read_by_id(chain_id)
            .map_err(LiteflowError::from)?;
        Ok(records
            .first()
            .map(|record| {
                vec![
                    record.el_data_md5.clone(),
                    record.node_instance_id_map_json.clone(),
                ]
            })
            .unwrap_or_default())
    }

    /// 参数化 upsert 节点实例编号记录。
    ///
    /// 对应 Java `writeInstanceIdFile(List, String, String)`。
    fn write_instance_id_file(
        &self,
        instance_id_list: &[InstanceInfoDto],
        el_md5: &str,
        chain_id: &str,
    ) -> LFResult<()> {
        self.jdbc_helper
            .execute_upsert(instance_id_list, el_md5, chain_id)
            .map_err(LiteflowError::from)
    }
}
