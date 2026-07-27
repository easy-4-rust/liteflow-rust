use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime};

use crate::exception::{LFResult, LiteflowError};
use crate::flow::FlowBus;
use crate::parser::{load_json_file, load_xml_file, load_yml_file};

/// 规则文件监听器，统一管理多路径监控、规则热刷新与后台任务清理。
///
/// Rust 端使用 Tokio 轮询替代 Apache Commons IO；每次变更先完整解析规则，
/// 成功后移除该文件已经删除的 Chain，解析失败则保留上一次可用规则。
///
/// 对应 Java: `com.yomahub.liteflow.monitor.MonitorFile`。
pub struct MonitorFile {
    flow_bus: FlowBus,
    paths: Arc<RwLock<HashSet<PathBuf>>>,
    monitors: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
    file_chains: Arc<Mutex<HashMap<PathBuf, HashSet<String>>>>,
}

impl MonitorFile {
    /// 返回当前 FlowBus 隔离域中的共享文件监听器。
    ///
    /// - `flow_bus`：接收规则变更的 LiteFlow 运行时。
    /// - 返回：同一 FlowBus 的克隆调用共享同一个 `MonitorFile`，不同 FlowBus
    ///   保持隔离。
    ///
    /// Java 使用 Hutool 进程级 Singleton；Rust 将单例范围收窄到 FlowBus，
    /// 避免测试和多租户运行时相互覆盖。对应 Java: `MonitorFile#getInstance()`。
    #[must_use]
    pub fn get_instance(flow_bus: FlowBus) -> Arc<Self> {
        let mut instance = flow_bus
            .monitor_file_instance
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(monitor_file) = instance.upgrade() {
            return monitor_file;
        }

        let monitor_file = Arc::new(Self::new(flow_bus.clone()));
        *instance = Arc::downgrade(&monitor_file);
        monitor_file
    }

    /// 创建绑定指定流程总线的文件监听器。
    ///
    /// `flow_bus` 接收规则热刷新结果；对应 Java 通过全局
    /// `FlowExecutorHolder` 间接取得同一运行时。
    #[must_use]
    pub fn new(flow_bus: FlowBus) -> Self {
        let paths = Arc::new(RwLock::new(HashSet::new()));
        let monitors = Arc::new(Mutex::new(Vec::<tokio::task::JoinHandle<()>>::new()));
        let file_chains = Arc::new(Mutex::new(HashMap::<PathBuf, HashSet<String>>::new()));
        let weak_paths = Arc::downgrade(&paths);
        let weak_monitors = Arc::downgrade(&monitors);
        let weak_file_chains = Arc::downgrade(&file_chains);

        // 只登记弱清理动作，避免 FlowBus 与 MonitorFile 相互强引用。
        flow_bus.register_monitor_file_cleaner(Arc::new(move || {
            if let Some(monitors) = weak_monitors.upgrade() {
                let mut monitors = monitors
                    .lock()
                    .map_err(|_| monitor_error("monitor task lock poisoned"))?;
                for monitor in monitors.drain(..) {
                    monitor.abort();
                }
            }
            if let Some(paths) = weak_paths.upgrade() {
                paths
                    .write()
                    .map_err(|_| monitor_error("monitor path lock poisoned"))?
                    .clear();
            }
            if let Some(file_chains) = weak_file_chains.upgrade() {
                file_chains
                    .lock()
                    .map_err(|_| monitor_error("monitor chain state lock poisoned"))?
                    .clear();
            }
            Ok(())
        }));

        Self {
            flow_bus,
            paths,
            monitors,
            file_chains,
        }
    }

    /// 添加一个规则文件或目录。
    ///
    /// 文件路径只监听该文件；目录在 `create` 时展开其中的 JSON、XML、YML/YAML
    /// 文件。对应 Java: `MonitorFile#addMonitorFilePath`。
    pub fn add_monitor_file_path(&self, path: impl AsRef<Path>) -> LFResult<()> {
        self.paths
            .write()
            .map_err(|_| monitor_error("monitor path lock poisoned"))?
            .insert(path.as_ref().to_path_buf());
        Ok(())
    }

    /// 批量添加规则文件或目录。
    ///
    /// `file_paths` 中的每一项都复用单路径校验。对应 Java:
    /// `MonitorFile#addMonitorFilePaths`。
    pub fn add_monitor_file_paths<I, P>(&self, file_paths: I) -> LFResult<()>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        for path in file_paths {
            self.add_monitor_file_path(path)?;
        }
        Ok(())
    }

    /// 创建后台监听任务。
    ///
    /// `interval` 是轮询间隔；重复调用不会创建第二组任务。首次创建会真实解析
    /// 所有规则文件并记录各自 Chain 集合。对应 Java: `MonitorFile#create`。
    pub fn create(&self, interval: Duration) -> LFResult<()> {
        let mut monitors = self
            .monitors
            .lock()
            .map_err(|_| monitor_error("monitor task lock poisoned"))?;
        if !monitors.is_empty() {
            return Ok(());
        }

        let files = self.rule_files()?;
        for path in &files {
            let chain_ids = load_rule_file(&self.flow_bus, path)?;
            self.file_chains
                .lock()
                .map_err(|_| monitor_error("monitor chain state lock poisoned"))?
                .insert(path.clone(), chain_ids);
        }

        for path in files {
            let flow_bus = self.flow_bus.clone();
            let file_chains = Arc::clone(&self.file_chains);
            monitors.push(tokio::spawn(async move {
                watch_file(flow_bus, file_chains, path, interval).await;
            }));
        }
        Ok(())
    }

    /// 处理规则文件修改事件并平滑替换该文件提供的 Chain。
    ///
    /// - `file`：发生修改的规则文件。
    /// - 返回：新规则完整解析成功时返回 `Ok(())`；失败时保留上一次成功记录。
    ///
    /// 对应 Java: `MonitorFile` 文件监听器的 `onFileChange(File)`。
    pub fn on_file_change(&self, file: impl AsRef<Path>) -> LFResult<()> {
        reload_file(&self.flow_bus, &self.file_chains, file.as_ref())
    }

    /// 处理规则文件删除事件并卸载该文件最后一次提供的全部 Chain。
    ///
    /// - `file`：已经删除的规则文件路径。
    /// - 返回：状态锁可用并完成卸载时返回 `Ok(())`。
    ///
    /// 对应 Java: `MonitorFile` 文件监听器的 `onFileDelete(File)`。
    pub fn on_file_delete(&self, file: impl AsRef<Path>) -> LFResult<()> {
        remove_file_chains(&self.flow_bus, &self.file_chains, file.as_ref())
    }

    /// 处理规则文件创建事件并装载文件中的全部 Chain。
    ///
    /// - `file`：新创建的规则文件。
    /// - 返回：规则完整解析并登记成功时返回 `Ok(())`。
    ///
    /// 对应 Java: `MonitorFile` 文件监听器的 `onFileCreate(File)`。
    pub fn on_file_create(&self, file: impl AsRef<Path>) -> LFResult<()> {
        reload_file(&self.flow_bus, &self.file_chains, file.as_ref())
    }

    /// 停止所有后台监听并清空路径配置。
    ///
    /// Tokio 任务通过 `abort` 可恢复地取消，不等待额外轮询周期。对应 Java:
    /// `MonitorFile#destroy`。
    pub fn destroy(&self) -> LFResult<()> {
        let mut monitors = self
            .monitors
            .lock()
            .map_err(|_| monitor_error("monitor task lock poisoned"))?;
        for monitor in monitors.drain(..) {
            monitor.abort();
        }
        self.paths
            .write()
            .map_err(|_| monitor_error("monitor path lock poisoned"))?
            .clear();
        self.file_chains
            .lock()
            .map_err(|_| monitor_error("monitor chain state lock poisoned"))?
            .clear();
        Ok(())
    }

    /// 判断是否仍有活动的文件监听任务。
    ///
    /// 对应 Java: `MonitorFile#isMonitoring`。
    #[must_use]
    pub fn is_monitoring(&self) -> bool {
        self.monitors
            .lock()
            .map(|monitors| {
                !monitors.is_empty() && monitors.iter().any(|monitor| !monitor.is_finished())
            })
            .unwrap_or(false)
    }

    fn rule_files(&self) -> LFResult<Vec<PathBuf>> {
        let paths = self
            .paths
            .read()
            .map_err(|_| monitor_error("monitor path lock poisoned"))?;
        let mut files = Vec::new();
        for path in paths.iter() {
            if path.is_dir() {
                let entries = std::fs::read_dir(path).map_err(|error| {
                    monitor_error(format!(
                        "read monitor directory {}: {error}",
                        path.display()
                    ))
                })?;
                files.extend(
                    entries
                        .filter_map(Result::ok)
                        .map(|entry| entry.path())
                        .filter(|entry| entry.is_file() && is_rule_file(entry)),
                );
            } else if path.is_file() && is_rule_file(path) {
                files.push(path.clone());
            } else {
                return Err(monitor_error(format!(
                    "unsupported monitor path {}",
                    path.display()
                )));
            }
        }
        files.sort();
        files.dedup();
        Ok(files)
    }
}

impl Drop for MonitorFile {
    fn drop(&mut self) {
        if let Ok(mut monitors) = self.monitors.lock() {
            for monitor in monitors.drain(..) {
                monitor.abort();
            }
        }
    }
}

async fn watch_file(
    flow_bus: FlowBus,
    file_chains: Arc<Mutex<HashMap<PathBuf, HashSet<String>>>>,
    path: PathBuf,
    interval: Duration,
) {
    let mut last_modified = modified_at(&path);
    loop {
        tokio::time::sleep(interval).await;
        let current_modified = modified_at(&path);
        if current_modified == last_modified {
            continue;
        }

        match current_modified {
            Some(_) => {
                // 只有完整解析成功后才替换文件所属 Chain 集合，避免坏规则污染运行时。
                if reload_file(&flow_bus, &file_chains, &path).is_ok() {
                    last_modified = current_modified;
                }
            }
            None => {
                // 文件删除时卸载该文件最后一次成功提供的全部 Chain。
                if remove_file_chains(&flow_bus, &file_chains, &path).is_ok() {
                    last_modified = None;
                }
            }
        }
    }
}

fn reload_file(
    flow_bus: &FlowBus,
    file_chains: &Mutex<HashMap<PathBuf, HashSet<String>>>,
    path: &Path,
) -> LFResult<()> {
    // 先完整解析新文件；解析失败时不触碰上一次成功的归属关系。
    let new_chain_ids = load_rule_file(flow_bus, path)?;
    let old_chain_ids = file_chains
        .lock()
        .map_err(|_| monitor_error("monitor chain state lock poisoned"))?
        .insert(path.to_path_buf(), new_chain_ids.clone())
        .unwrap_or_default();

    // 新规则发布成功后才删除已经从该文件消失的 Chain，保证热更新平滑。
    for stale_chain_id in old_chain_ids.difference(&new_chain_ids) {
        flow_bus.remove_chain(stale_chain_id);
    }
    Ok(())
}

fn remove_file_chains(
    flow_bus: &FlowBus,
    file_chains: &Mutex<HashMap<PathBuf, HashSet<String>>>,
    path: &Path,
) -> LFResult<()> {
    let chain_ids = file_chains
        .lock()
        .map_err(|_| monitor_error("monitor chain state lock poisoned"))?
        .remove(path)
        .unwrap_or_default();
    for chain_id in chain_ids {
        flow_bus.remove_chain(&chain_id);
    }
    Ok(())
}

fn load_rule_file(flow_bus: &FlowBus, path: &Path) -> LFResult<HashSet<String>> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let chain_ids = match extension.as_str() {
        "json" => load_json_file(flow_bus, path)?,
        "xml" => load_xml_file(flow_bus, path)?,
        "yml" | "yaml" => load_yml_file(flow_bus, path)?,
        _ => {
            return Err(monitor_error(format!(
                "unsupported rule file {}",
                path.display()
            )));
        }
    };
    Ok(chain_ids.into_iter().collect())
}

fn is_rule_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("json" | "xml" | "yml" | "yaml")
    )
}

fn modified_at(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
}

fn monitor_error(message: impl Into<String>) -> LiteflowError {
    LiteflowError::MonitorFileInitError(message.into())
}
