//! 节点实例编号 SPI 的公共基座逻辑。

use std::collections::HashMap;

use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::node::Node;
use crate::flow::entity::InstanceInfoDto;

use super::NodeInstanceIdManageSpi;

/// 提供实例编号文件解析、节点查询、编号生成与恢复的公共算法。
///
/// Rust 使用无状态对象加显式节点切片替代 Java 对 `FlowBus` 全局静态表的隐式
/// 访问；算法和出现下标均保持一致。
///
/// 对应 Java:
/// `com.yomahub.liteflow.flow.instanceId.BaseNodeInstanceIdManageSpi`。
#[derive(Debug, Default, Clone, Copy)]
pub struct BaseNodeInstanceIdManageSpi;

impl BaseNodeInstanceIdManageSpi {
    /// 解析实例编号文件第二行开始的 JSON 数组。
    pub fn parse_instance_infos(lines: &[String]) -> LFResult<Vec<InstanceInfoDto>> {
        let mut result = Vec::new();
        for line in lines.iter().skip(1).filter(|line| !line.trim().is_empty()) {
            let mut current: Vec<InstanceInfoDto> =
                serde_json::from_str(line).map_err(|error| {
                    LiteflowError::Custom(format!("parse node instance id file failed: {error}"))
                })?;
            result.append(&mut current);
        }
        Ok(result)
    }

    /// 根据实例 id 返回节点出现位置；未命中返回 `-1`。
    ///
    /// 对应 Java: `BaseNodeInstanceIdManageSpi#getNodeLocationById`。
    pub fn get_node_location_by_id(lines: &[String], instance_id: &str) -> LFResult<isize> {
        if instance_id.trim().is_empty() {
            return Ok(-1);
        }
        Ok(Self::parse_instance_infos(lines)?
            .into_iter()
            .find(|info| info.instance_id() == Some(instance_id))
            .and_then(|info| info.index())
            .and_then(|index| isize::try_from(index).ok())
            .unwrap_or(-1))
    }

    /// 返回指定节点 id 的全部实例 id。
    ///
    /// 对应 Java: `BaseNodeInstanceIdManageSpi#getNodeInstanceIds`。
    pub fn get_node_instance_ids(lines: &[String], node_id: &str) -> LFResult<Vec<String>> {
        if node_id.trim().is_empty() {
            return Ok(Vec::new());
        }
        Ok(Self::parse_instance_infos(lines)?
            .into_iter()
            .filter(|info| info.node_id() == Some(node_id))
            .filter_map(|info| info.instance_id().map(ToOwned::to_owned))
            .collect())
    }

    /// 根据实例 id 从显式节点列表中查找节点。
    ///
    /// 对应 Java: `BaseNodeInstanceIdManageSpi#getNodeByIdAndInstanceId`。
    pub fn get_node_by_id_and_instance_id<'a>(
        nodes: &'a [Node],
        instance_id: &str,
    ) -> Option<&'a Node> {
        if instance_id.trim().is_empty() {
            return None;
        }
        nodes
            .iter()
            .find(|node| node.node_instance_id() == Some(instance_id))
    }

    /// 根据节点 id 和从零开始的出现下标查找节点。
    ///
    /// 对应 Java: `BaseNodeInstanceIdManageSpi#getNodeByIdAndIndex`。
    pub fn get_node_by_id_and_index<'a>(
        nodes: &'a [Node],
        node_id: &str,
        index: usize,
    ) -> Option<&'a Node> {
        nodes
            .iter()
            .filter(|node| node.node_ref().id == node_id)
            .nth(index)
    }

    /// 为节点列表生成实例信息并把 id 回写到 Node。
    ///
    /// 对应 Java: `BaseNodeInstanceIdManageSpi#writeNodeInstanceId` 与
    /// `#addInstanceIdFromExecutableGroup`。
    pub fn assign_instance_ids(
        spi: &dyn NodeInstanceIdManageSpi,
        nodes: &mut [Node],
        chain_id: &str,
    ) -> Vec<InstanceInfoDto> {
        let mut occurrences = HashMap::<String, usize>::new();
        nodes
            .iter_mut()
            .map(|node| {
                let node_id = node.node_ref().id.clone();
                let occurrence = occurrences.entry(node_id.clone()).or_default();
                let info = spi.build_instance_info(chain_id, &node_id, *occurrence);
                *occurrence += 1;
                if let Some(instance_id) = info.instance_id() {
                    node.set_node_instance_id(instance_id);
                }
                info
            })
            .collect()
    }

    /// 按 chainId、nodeId、index 从文件信息恢复 Node 的实例 id。
    ///
    /// 对应 Java: `BaseNodeInstanceIdManageSpi#setInstanceIdFromFile`。
    pub fn restore_instance_ids(nodes: &mut [Node], chain_id: &str, infos: &[InstanceInfoDto]) {
        let mut occurrences = HashMap::<String, usize>::new();
        for node in nodes {
            let node_id = node.node_ref().id.clone();
            let occurrence = occurrences.entry(node_id.clone()).or_default();
            if let Some(info) = infos.iter().find(|info| {
                info.chain_id() == Some(chain_id)
                    && info.node_id() == Some(node_id.as_str())
                    && info.index() == Some(*occurrence)
            }) {
                if let Some(instance_id) = info.instance_id() {
                    node.set_node_instance_id(instance_id);
                }
            }
            *occurrence += 1;
        }
    }
}
