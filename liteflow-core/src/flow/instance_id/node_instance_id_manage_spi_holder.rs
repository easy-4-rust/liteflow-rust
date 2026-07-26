//! 节点实例编号 SPI 持有器。

use std::sync::{Arc, RwLock};

use super::{DefaultNodeInstanceIdManageSpiImpl, NodeInstanceIdManageSpi};

/// 在一个 `FlowBus` 生命周期内共享并可替换实例编号 SPI。
///
/// Java 使用进程级 singleton + ServiceLoader；Rust 使用可克隆 holder + 显式注册，
/// 既保留运行期替换能力，也避免多个 FlowBus 相互污染。
///
/// 对应 Java:
/// `com.yomahub.liteflow.flow.instanceId.NodeInstanceIdManageSpiHolder`。
#[derive(Clone)]
pub struct NodeInstanceIdManageSpiHolder {
    instance: Arc<RwLock<Arc<dyn NodeInstanceIdManageSpi>>>,
}

impl NodeInstanceIdManageSpiHolder {
    /// 使用指定 SPI 创建持有器。
    #[must_use]
    pub fn new(instance: Arc<dyn NodeInstanceIdManageSpi>) -> Self {
        Self {
            instance: Arc::new(RwLock::new(instance)),
        }
    }

    /// 获取当前 SPI。
    ///
    /// 对应 Java: `NodeInstanceIdManageSpiHolder#getNodeInstanceIdManageSpi`。
    #[must_use]
    pub fn get_node_instance_id_manage_spi(&self) -> Arc<dyn NodeInstanceIdManageSpi> {
        self.instance.read().unwrap().clone()
    }

    /// 替换当前 SPI。
    ///
    /// 对应 Java: `NodeInstanceIdManageSpiHolder#setNodeInstanceIdManageSpi`。
    pub fn set_node_instance_id_manage_spi(&self, instance: Arc<dyn NodeInstanceIdManageSpi>) {
        *self.instance.write().unwrap() = instance;
    }
}

impl Default for NodeInstanceIdManageSpiHolder {
    fn default() -> Self {
        Self::new(Arc::new(DefaultNodeInstanceIdManageSpiImpl::default()))
    }
}
