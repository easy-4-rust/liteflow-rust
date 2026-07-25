//! 对应 flow.instanceId 包：同一节点在一条链中多次出现时的实例编号管理。
//!
//! Java 默认实现把实例编号写入本地文件（跨热刷新保持稳定）；
//! Rust 版提供 trait + 内存默认实现（文件持久化见迁移对照表 🔶）。

use std::collections::HashMap;
use std::sync::Mutex;

/// 对应 NodeInstanceIdManageSpi
pub trait NodeInstanceIdManageSpi: Send + Sync + 'static {
    /// 为 chain 中第 occurrence 次出现的 node_id 生成实例编号
    fn gen_instance_id(&self, chain_id: &str, node_id: &str, occurrence: usize) -> String;
}

/// 默认内存实现：nodeId_chainId_occurrence
pub struct DefaultNodeInstanceIdManageSpi {
    cache: Mutex<HashMap<String, String>>,
}

impl Default for DefaultNodeInstanceIdManageSpi {
    fn default() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }
}

impl NodeInstanceIdManageSpi for DefaultNodeInstanceIdManageSpi {
    fn gen_instance_id(&self, chain_id: &str, node_id: &str, occurrence: usize) -> String {
        let key = format!("{chain_id}:{node_id}:{occurrence}");
        let mut cache = self.cache.lock().unwrap();
        cache
            .entry(key)
            .or_insert_with(|| {
                if occurrence == 0 {
                    node_id.to_string()
                } else {
                    format!("{node_id}_{occurrence}")
                }
            })
            .clone()
    }
}
