//! 对应 monitor.MonitorBus + MonitorFile 的"平滑热刷新"：
//! 轮询文件 mtime，变更后先完整解析，再原子替换链路表；解析失败不影响在跑链路。

use super::el::load_json_file;
use crate::exception::LFResult;
use crate::flow::flow_bus::FlowBus;
use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

pub struct RuleWatcher {
    bus: FlowBus,
    path: PathBuf,
    chain_ids: Arc<DashMap<String, ()>>,
}

impl RuleWatcher {
    pub fn new(bus: FlowBus, path: impl AsRef<Path>) -> LFResult<Self> {
        let ids = load_json_file(&bus, path.as_ref())?;
        let chain_ids = Arc::new(DashMap::new());
        for id in ids {
            chain_ids.insert(id, ());
        }
        Ok(Self {
            bus,
            path: path.as_ref().to_path_buf(),
            chain_ids,
        })
    }

    fn mtime(&self) -> Option<SystemTime> {
        std::fs::metadata(&self.path)
            .ok()
            .and_then(|m| m.modified().ok())
    }

    /// 启动后台轮询（abort 返回的 JoinHandle 即停止）
    pub fn watch(self, interval: Duration) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut last = self.mtime();
            loop {
                tokio::time::sleep(interval).await;
                let cur = self.mtime();
                if cur.is_some() && cur != last {
                    last = cur;
                    match load_json_file(&self.bus, &self.path) {
                        Ok(ids) => {
                            let new_set: std::collections::HashSet<&String> = ids.iter().collect();
                            let stale: Vec<String> = self
                                .chain_ids
                                .iter()
                                .map(|r| r.key().clone())
                                .filter(|id| !new_set.contains(id))
                                .collect();
                            for id in stale {
                                self.bus.remove_chain(&id);
                                self.chain_ids.remove(&id);
                            }
                            for id in ids {
                                self.chain_ids.insert(id, ());
                            }
                            println!("[liteflow] rule file {} reloaded", self.path.display());
                        }
                        Err(e) => {
                            eprintln!("[liteflow] reload {} failed: {e}", self.path.display());
                        }
                    }
                }
            }
        })
    }
}
