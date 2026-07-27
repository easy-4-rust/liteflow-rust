//! 节点实例编号传输对象。
//!
//! 对应 Java: `com.yomahub.liteflow.flow.entity.InstanceInfoDto`。

/// 描述某个节点在指定 Chain 中第几次出现及其稳定实例编号。
///
/// JSON 字段保持 Java/Jackson 的 camelCase：
/// `{"chainId":"chain1","nodeId":"a","instanceId":"a_xxx_0","index":0}`。
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceInfoDto {
    chain_id: Option<String>,
    node_id: Option<String>,
    instance_id: Option<String>,
    index: Option<usize>,
}

impl InstanceInfoDto {
    /// 创建完整实例信息。
    pub fn new(
        chain_id: impl Into<String>,
        node_id: impl Into<String>,
        instance_id: impl Into<String>,
        index: usize,
    ) -> Self {
        Self {
            chain_id: Some(chain_id.into()),
            node_id: Some(node_id.into()),
            instance_id: Some(instance_id.into()),
            index: Some(index),
        }
    }

    /// 返回 Chain ID。对应 Java `getChainId()`。
    pub fn chain_id(&self) -> Option<&str> {
        self.chain_id.as_deref()
    }

    /// 返回 Chain ID。
    ///
    /// # 返回
    /// Java 字段尚未赋值时返回 `None`，否则返回真实字段的字符串切片。
    ///
    /// 对应 Java: `InstanceInfoDto#getChainId`。
    #[must_use]
    pub fn get_chain_id(&self) -> Option<&str> {
        self.chain_id()
    }

    /// 设置 Chain ID。对应 Java `setChainId(String)`。
    pub fn set_chain_id(&mut self, chain_id: impl Into<String>) {
        self.chain_id = Some(chain_id.into());
    }

    /// 返回节点 ID。对应 Java `getNodeId()`。
    pub fn node_id(&self) -> Option<&str> {
        self.node_id.as_deref()
    }

    /// 返回节点 ID。
    ///
    /// # 返回
    /// Java 字段尚未赋值时返回 `None`，否则返回真实字段的字符串切片。
    ///
    /// 对应 Java: `InstanceInfoDto#getNodeId`。
    #[must_use]
    pub fn get_node_id(&self) -> Option<&str> {
        self.node_id()
    }

    /// 设置节点 ID。对应 Java `setNodeId(String)`。
    pub fn set_node_id(&mut self, node_id: impl Into<String>) {
        self.node_id = Some(node_id.into());
    }

    /// 返回节点实例编号。对应 Java `getInstanceId()`。
    pub fn instance_id(&self) -> Option<&str> {
        self.instance_id.as_deref()
    }

    /// 返回节点实例编号。
    ///
    /// # 返回
    /// Java 字段尚未赋值时返回 `None`，否则返回稳定实例编号。
    ///
    /// 对应 Java: `InstanceInfoDto#getInstanceId`。
    #[must_use]
    pub fn get_instance_id(&self) -> Option<&str> {
        self.instance_id()
    }

    /// 设置节点实例编号。对应 Java `setInstanceId(String)`。
    pub fn set_instance_id(&mut self, instance_id: impl Into<String>) {
        self.instance_id = Some(instance_id.into());
    }

    /// 返回同名节点在 Chain 中的出现下标。对应 Java `getIndex()`。
    pub fn index(&self) -> Option<usize> {
        self.index
    }

    /// 返回同名节点在 Chain 中的出现下标。
    ///
    /// # 返回
    /// Java `Integer` 未赋值时返回 `None`，否则返回真实下标。
    ///
    /// 对应 Java: `InstanceInfoDto#getIndex`。
    #[must_use]
    pub fn get_index(&self) -> Option<usize> {
        self.index()
    }

    /// 设置出现下标。对应 Java `setIndex(Integer)`。
    pub fn set_index(&mut self, index: usize) {
        self.index = Some(index);
    }
}
