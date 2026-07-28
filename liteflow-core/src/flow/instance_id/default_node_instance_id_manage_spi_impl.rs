//! 默认节点实例编号管理实现。

use std::fs;
use std::path::{Path, PathBuf};

use crate::common::ChainConstant;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::entity::InstanceInfoDto;
use crate::util::SerialsUtil;

use super::NodeInstanceIdManageSpi;

/// 把节点实例信息持久化到工作目录 `.node_instance_id/<chainId>`。
///
/// 对应 Java:
/// `com.yomahub.liteflow.flow.instanceId.DefaultNodeInstanceIdManageSpiImpl`。
pub struct DefaultNodeInstanceIdManageSpiImpl {
    base_path: PathBuf,
}

impl DefaultNodeInstanceIdManageSpiImpl {
    /// 使用指定目录创建默认实现，主要用于隔离测试或嵌入式运行。
    #[must_use]
    pub fn with_base_path(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }

    /// 返回实例编号文件基础目录。
    #[must_use]
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    /// 读取指定 Chain 的 EL 摘要与实例编号 JSON。
    ///
    /// 参数 `chain_id` 对应 Java 同名参数；空白或文件不存在时返回空集合。
    /// 对应 Java: `DefaultNodeInstanceIdManageSpiImpl#readInstanceIdFile`。
    pub fn read_instance_id_file(&self, chain_id: &str) -> LFResult<Vec<String>> {
        <Self as NodeInstanceIdManageSpi>::read_instance_id_file(self, chain_id)
    }

    /// 按 Java 两行格式写入节点实例编号文件。
    ///
    /// 第一行为 `el_md5`，第二行为 `instance_id_list` 的 serde JSON；参数名称与
    /// Java 一致。对应 Java:
    /// `DefaultNodeInstanceIdManageSpiImpl#writeInstanceIdFile`。
    pub fn write_instance_id_file(
        &self,
        instance_id_list: &[InstanceInfoDto],
        el_md5: &str,
        chain_id: &str,
    ) -> LFResult<()> {
        <Self as NodeInstanceIdManageSpi>::write_instance_id_file(
            self,
            instance_id_list,
            el_md5,
            chain_id,
        )
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
    /// 为当前 EL 快照生成 `nodeId_shortUuid_index` 格式的新实例编号。
    ///
    /// 参数 `chain_id` 标识所属 Chain，`node_id` 是组件 ID，`occurrence` 是同名
    /// 节点在主体 Condition 中从零开始的出现下标。稳定编号由持久化快照恢复，
    /// 只有 EL 变化时才会调用本方法重新生成。
    /// 对应 Java: `BaseNodeInstanceIdManageSpi#addInstanceIdFromExecutableGroup`。
    fn gen_instance_id(&self, chain_id: &str, node_id: &str, occurrence: usize) -> String {
        let _ = chain_id;
        // 稳定性由 EL MD5 对应的持久化快照提供；EL 变化时必须像 Java 一样重新
        // 生成短 UUID，不能复用进程内 `(chain,node,index)` 缓存。
        let short_uuid = SerialsUtil::generate_short_uuid();
        format!("{node_id}_{short_uuid}_{occurrence}")
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
