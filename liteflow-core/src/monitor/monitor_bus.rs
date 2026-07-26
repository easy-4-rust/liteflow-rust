use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use dashmap::DashMap;

use super::comp_statistics::CompStatistics;
use super::stat_entry::StatEntry;
use crate::slot::DataBus;
use crate::util::BoundedPriorityBlockingQueue;

const DEFAULT_QUEUE_LIMIT: usize = 200;

/// 组件执行统计总线。
///
/// 每个组件保留有限条最新 `CompStatistics`，报表平均值只基于这些有界样本；
/// 同时累计成功/失败次数，延续 Rust 运行时已有的诊断能力。
///
/// 对应 Java: `com.yomahub.liteflow.monitor.MonitorBus`。
pub struct MonitorBus {
    stats: DashMap<String, StatEntry>,
    statistics_map: DashMap<String, Arc<BoundedPriorityBlockingQueue<CompStatistics>>>,
    queue_limit: AtomicUsize,
}

impl Default for MonitorBus {
    fn default() -> Self {
        Self {
            stats: DashMap::new(),
            statistics_map: DashMap::new(),
            queue_limit: AtomicUsize::new(DEFAULT_QUEUE_LIMIT),
        }
    }
}

impl MonitorBus {
    /// 使用 Java 默认样本上限 200 创建监控总线。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 使用指定样本上限创建监控总线。
    ///
    /// 参数 `queue_limit` 对应 `LiteflowConfig#getQueueLimit`。
    #[must_use]
    pub fn with_queue_limit(queue_limit: usize) -> Self {
        let monitor_bus = Self::new();
        monitor_bus.set_queue_limit(queue_limit);
        monitor_bus
    }

    /// 记录一次组件执行并写入有界最新样本队列。
    ///
    /// 参数 `node_id` 为组件展示名，`time_spent` 为执行耗时，`success` 表示
    /// 本次执行是否成功。对应 Java `NodeComponent#execute` 中的性能统计路径。
    pub fn record(&self, node_id: &str, time_spent: Duration, success: bool) {
        let entry = self.stats.entry(node_id.to_string()).or_default();
        entry.total.fetch_add(1, Ordering::Relaxed);
        if success {
            entry.success.fetch_add(1, Ordering::Relaxed);
        } else {
            entry.fail.fetch_add(1, Ordering::Relaxed);
        }
        let milliseconds = time_spent.as_millis() as u64;
        entry
            .total_time_ms
            .fetch_add(milliseconds, Ordering::Relaxed);
        entry.max_time_ms.fetch_max(milliseconds, Ordering::Relaxed);
        self.add_statistics(CompStatistics::new(node_id, milliseconds));
    }

    /// 将一条统计记录加入对应组件的有界队列。
    ///
    /// 对应 Java: `MonitorBus#addStatistics`。
    pub fn add_statistics(&self, statistics: CompStatistics) {
        let component_name = statistics.component_clazz_name().to_string();
        let queue = self
            .statistics_map
            .entry(component_name)
            .or_insert_with(|| {
                Arc::new(BoundedPriorityBlockingQueue::new(
                    self.queue_limit.load(Ordering::Relaxed),
                ))
            })
            .clone();
        queue.offer(statistics);
    }

    /// 返回累计次数与有界最新样本耗时组成的统计报表。
    ///
    /// 报表按累计执行次数降序排列。对应 Java `MonitorBus#printStatistics`
    /// 计算每个组件平均耗时的阶段。
    #[must_use]
    pub fn report(&self) -> Vec<CompStatistics> {
        let mut output: Vec<CompStatistics> = self
            .stats
            .iter()
            .map(|entry| {
                let samples = self
                    .statistics_map
                    .get(entry.key())
                    .map(|queue| queue.to_list())
                    .unwrap_or_default();
                let average = if samples.is_empty() {
                    entry.avg_time_ms()
                } else {
                    samples.iter().map(CompStatistics::time_spent).sum::<u64>()
                        / samples.len() as u64
                };
                let maximum = samples
                    .iter()
                    .map(CompStatistics::time_spent)
                    .max()
                    .unwrap_or_else(|| entry.max_time_ms.load(Ordering::Relaxed));
                CompStatistics::aggregate(
                    entry.key().clone(),
                    entry.total.load(Ordering::Relaxed),
                    entry.success.load(Ordering::Relaxed),
                    entry.fail.load(Ordering::Relaxed),
                    average,
                    maximum,
                )
            })
            .collect();
        output.sort_by(|left, right| right.total.cmp(&left.total));
        output
    }

    /// 生成 Java 定时任务打印的统计文本。
    ///
    /// 返回值便于宿主接入 tracing、日志文件或测试捕获。
    /// 对应 Java: `MonitorBus#printStatistics`。
    #[must_use]
    pub fn print_statistics(&self) -> String {
        let mut report = String::from(
            "以下为LiteFlow中间件统计信息：\n\
             ======================================================================================\n\
             ===================================SLOT INFO==========================================\n",
        );
        report.push_str(&format!(
            "SLOT TOTAL SIZE : {}\nSLOT OCCUPY COUNT : {}\n",
            DataBus::total_size(),
            DataBus::occupy_count()
        ));
        report.push_str(
            "===============================TIME AVERAGE SPENT=====================================\n",
        );
        let mut statistics = self
            .statistics_map()
            .into_iter()
            .map(|(component_name, samples)| {
                let average = if samples.is_empty() {
                    0.0
                } else {
                    samples.iter().map(CompStatistics::time_spent).sum::<u64>() as f64
                        / samples.len() as f64
                };
                (component_name, average)
            })
            .collect::<Vec<_>>();
        statistics.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for (component_name, average) in statistics {
            report.push_str(&format!(
                "COMPONENT[{component_name}] AVERAGE TIME SPENT : {average:.2}\n"
            ));
        }
        report.push_str(
            "======================================================================================",
        );
        report
    }

    /// 返回每个组件当前保留的有序样本快照。
    ///
    /// 对应 Java: `MonitorBus#getStatisticsMap`。
    #[must_use]
    pub fn statistics_map(&self) -> HashMap<String, Vec<CompStatistics>> {
        self.statistics_map
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().to_list()))
            .collect()
    }

    /// 设置新建组件样本队列的容量。
    ///
    /// 已存在队列保持原容量，与 Java 替换配置后只影响后续新队列的行为一致。
    pub fn set_queue_limit(&self, queue_limit: usize) {
        self.queue_limit.store(queue_limit, Ordering::Relaxed);
    }

    /// 返回当前新建队列使用的容量。
    #[must_use]
    pub fn queue_limit(&self) -> usize {
        self.queue_limit.load(Ordering::Relaxed)
    }

    /// 清空累计统计和全部样本队列。
    pub fn clear(&self) {
        self.stats.clear();
        self.statistics_map.clear();
    }
}
