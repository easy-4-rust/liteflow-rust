//! 节点实例编号查询结果。

use serde::{Deserialize, Serialize};

/// 保存指定 Chain 的 EL 摘要和节点实例编号 JSON。
///
/// 对应 Java: `com.yomahub.liteflow.parser.sql.read.vo.InstanceIdVO`。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceIdVO {
    /// Chain id。
    pub chain_id: String,
    /// EL 数据摘要。
    pub el_data_md5: String,
    /// 节点实例编号映射 JSON。
    pub node_instance_id_map_json: String,
}
