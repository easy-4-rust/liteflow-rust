//! 对应 Java: `com.yomahub.liteflow.flow.instanceId.NodeInstanceIdManageSpi`。

use crate::exception::LFResult;
use crate::flow::entity::InstanceInfoDto;

use super::BaseNodeInstanceIdManageSpi;

/// 同一节点在一条链中多次出现时的实例编号管理协议。
pub trait NodeInstanceIdManageSpi: Send + Sync + 'static {
    /// 为链中第 `occurrence` 次出现的节点生成实例编号。
    fn gen_instance_id(&self, chain_id: &str, node_id: &str, occurrence: usize) -> String;

    /// 生成完整实例信息。
    ///
    /// 对应 Java `BaseNodeInstanceIdManageSpi#addInstanceIdFromExecutableGroup`：
    /// 生成 instanceId 的同时保存 chainId、nodeId 与出现下标。
    fn build_instance_info(
        &self,
        chain_id: &str,
        node_id: &str,
        occurrence: usize,
    ) -> InstanceInfoDto {
        InstanceInfoDto::new(
            chain_id,
            node_id,
            self.gen_instance_id(chain_id, node_id, occurrence),
            occurrence,
        )
    }

    /// 读取实例编号文件，第一行为 EL MD5，后续行为 DTO JSON 数组。
    ///
    /// 对应 Java: `NodeInstanceIdManageSpi#readInstanceIdFile`。
    fn read_instance_id_file(&self, _chain_id: &str) -> LFResult<Vec<String>> {
        Ok(Vec::new())
    }

    /// 写入实例编号文件。
    ///
    /// 对应 Java: `NodeInstanceIdManageSpi#writeInstanceIdFile`。
    fn write_instance_id_file(
        &self,
        _instance_id_list: &[InstanceInfoDto],
        _el_md5: &str,
        _chain_id: &str,
    ) -> LFResult<()> {
        Ok(())
    }

    /// 根据实例 id 返回出现位置，未命中返回 `-1`。
    ///
    /// 对应 Java: `NodeInstanceIdManageSpi#getNodeLocationById`。
    fn get_node_location_by_id(&self, chain_id: &str, instance_id: &str) -> LFResult<isize> {
        let lines = self.read_instance_id_file(chain_id)?;
        BaseNodeInstanceIdManageSpi::get_node_location_by_id(&lines, instance_id)
    }

    /// 根据节点 id 返回全部实例 id。
    ///
    /// 对应 Java: `NodeInstanceIdManageSpi#getNodeInstanceIds`。
    fn get_node_instance_ids(&self, chain_id: &str, node_id: &str) -> LFResult<Vec<String>> {
        let lines = self.read_instance_id_file(chain_id)?;
        BaseNodeInstanceIdManageSpi::get_node_instance_ids(&lines, node_id)
    }
}
