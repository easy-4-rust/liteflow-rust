//! SQL 轮询对账公共算法。

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use sha1::{Digest, Sha1};

/// 保存对象唯一键到内容 SHA-1 的快照，并计算新增、修改和删除集合。
///
/// 对应 Java:
/// `com.yomahub.liteflow.parser.sql.polling.AbstractSqlReadPollTask`。
#[derive(Debug, Default)]
pub struct AbstractSqlReadPollTask {
    data_sha_map: Mutex<HashMap<String, String>>,
}

impl AbstractSqlReadPollTask {
    /// 创建空轮询快照。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 初始化摘要快照。对应 Java `initData`。
    pub fn init_data<T>(
        &self,
        data_list: &[T],
        get_key: impl Fn(&T) -> String,
        get_value: impl Fn(&T) -> String,
        get_ext_value: impl Fn(&T) -> Option<String>,
    ) {
        let snapshot = data_list
            .iter()
            .map(|data| {
                (
                    get_key(data),
                    fingerprint(&get_value(data), get_ext_value(data).as_deref()),
                )
            })
            .collect();
        *self.data_sha_map.lock().expect("SQL 轮询摘要锁中毒") = snapshot;
    }

    /// 对比新快照，返回需保存对象和需删除 id。
    ///
    /// 对应 Java `AbstractSqlReadPollTask#execute` 中的增删改判定。
    pub fn diff<T: Clone>(
        &self,
        data_list: &[T],
        get_key: impl Fn(&T) -> String,
        get_value: impl Fn(&T) -> String,
        get_ext_value: impl Fn(&T) -> Option<String>,
    ) -> (Vec<T>, Vec<String>) {
        let mut snapshot = self.data_sha_map.lock().expect("SQL 轮询摘要锁中毒");
        let mut save_elements = Vec::new();
        let mut current_ids = HashSet::new();

        for data in data_list {
            let id = get_key(data);
            let digest = fingerprint(&get_value(data), get_ext_value(data).as_deref());
            current_ids.insert(id.clone());
            if snapshot.get(&id) != Some(&digest) {
                save_elements.push(data.clone());
                snapshot.insert(id, digest);
            }
        }

        let delete_ids = snapshot
            .keys()
            .filter(|id| !current_ids.contains(*id))
            .cloned()
            .collect::<Vec<_>>();
        for id in &delete_ids {
            snapshot.remove(id);
        }
        (save_elements, delete_ids)
    }
}

fn fingerprint(value: &str, ext_value: Option<&str>) -> String {
    let mut hasher = Sha1::new();
    hasher.update(value.as_bytes());
    if let Some(ext_value) = ext_value.filter(|value| !value.trim().is_empty()) {
        hasher.update(b"|||");
        hasher.update(ext_value.as_bytes());
    }
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
