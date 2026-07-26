//! 默认节点实例编号管理实现。

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rand::distributions::{Alphanumeric, DistString};

use crate::common::ChainConstant;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::entity::InstanceInfoDto;

use super::NodeInstanceIdManageSpi;

/// 把节点实例信息持久化到工作目录 `.node_instance_id/<chainId>`。
///
/// 对应 Java:
/// `com.yomahub.liteflow.flow.instanceId.DefaultNodeInstanceIdManageSpiImpl`。
pub struct DefaultNodeInstanceIdManageSpiImpl {
    base_path: PathBuf,
    cache: Mutex<HashMap<String, String>>,
}

impl DefaultNodeInstanceIdManageSpiImpl {
    /// 使用指定目录创建默认实现，主要用于隔离测试或嵌入式运行。
    #[must_use]
    pub fn with_base_path(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// 返回实例编号文件基础目录。
    #[must_use]
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    fn chain_path(&self, chain_id: &str) -> PathBuf {
        self.base_path.join(chain_id)
    }
}

impl Default for DefaultNodeInstanceIdManageSpiImpl {
    fn default() -> Self {
        let base_path = std::env::current_dir()
            .unwrap_or_default()
            .join(ChainConstant::NODE_INSTANCE_PATH);
        Self::with_base_path(base_path)
    }
}

impl NodeInstanceIdManageSpi for DefaultNodeInstanceIdManageSpiImpl {
    fn gen_instance_id(&self, chain_id: &str, node_id: &str, occurrence: usize) -> String {
        let key = format!("{chain_id}:{node_id}:{occurrence}");
        let mut cache = self.cache.lock().unwrap();
        cache
            .entry(key)
            .or_insert_with(|| {
                let short_uuid = Alphanumeric.sample_string(&mut rand::thread_rng(), 8);
                format!("{node_id}_{short_uuid}_{occurrence}")
            })
            .clone()
    }

    fn read_instance_id_file(&self, chain_id: &str) -> LFResult<Vec<String>> {
        if chain_id.trim().is_empty() {
            return Ok(Vec::new());
        }
        let path = self.chain_path(chain_id);
        if !path.is_file() {
            return Ok(Vec::new());
        }
        fs::read_to_string(&path)
            .map(|content| content.lines().map(ToOwned::to_owned).collect())
            .map_err(|error| {
                LiteflowError::Custom(format!(
                    "read node instance id file[{}] failed: {error}",
                    path.display()
                ))
            })
    }

    fn write_instance_id_file(
        &self,
        instance_id_list: &[InstanceInfoDto],
        el_md5: &str,
        chain_id: &str,
    ) -> LFResult<()> {
        if chain_id.trim().is_empty() || instance_id_list.is_empty() {
            return Ok(());
        }
        fs::create_dir_all(&self.base_path).map_err(|error| {
            LiteflowError::Custom(format!(
                "create node instance id directory[{}] failed: {error}",
                self.base_path.display()
            ))
        })?;
        let json = serde_json::to_string(instance_id_list).map_err(|error| {
            LiteflowError::Custom(format!("serialize node instance ids failed: {error}"))
        })?;
        let path = self.chain_path(chain_id);
        fs::write(&path, format!("{el_md5}\n{json}\n")).map_err(|error| {
            LiteflowError::Custom(format!(
                "write node instance id file[{}] failed: {error}",
                path.display()
            ))
        })
    }
}
