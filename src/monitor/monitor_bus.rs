//! 对应 MonitorBus + CompStatistics。

use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// 单个组件的统计项（对应 CompStatistics 聚合）
#[derive(Debug, Default)]
pub struct StatEntry {
    pub total: AtomicU64,
    pub success: AtomicU64,
    pub fail: AtomicU64,
    pub total_time_ms: AtomicU64,
    pub max_time_ms: AtomicU64,
}

impl StatEntry {
    pub fn avg_time_ms(&self) -> u64 {
        let total = self.total.load(Ordering::Relaxed);
        if total == 0 {
            0
        } else {
            self.total_time_ms.load(Ordering::Relaxed) / total
        }
    }
}

/// 对应 MonitorBus
#[derive(Default)]
pub struct MonitorBus {
    stats: DashMap<String, StatEntry>,
}

impl MonitorBus {
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一次组件执行（对应 publishStatistics）
    pub fn record(&self, node_id: &str, time_spent: Duration, success: bool) {
        let entry = self.stats.entry(node_id.to_string()).or_default();
        entry.total.fetch_add(1, Ordering::Relaxed);
        if success {
            entry.success.fetch_add(1, Ordering::Relaxed);
        } else {
            entry.fail.fetch_add(1, Ordering::Relaxed);
        }
        let ms = time_spent.as_millis() as u64;
        entry.total_time_ms.fetch_add(ms, Ordering::Relaxed);
        entry.max_time_ms.fetch_max(ms, Ordering::Relaxed);
    }

    /// 统计报表（对应定时打印的统计信息）
    pub fn report(&self) -> Vec<CompStatistics> {
        let mut out: Vec<CompStatistics> = self
            .stats
            .iter()
            .map(|r| CompStatistics {
                node_id: r.key().clone(),
                total: r.total.load(Ordering::Relaxed),
                success: r.success.load(Ordering::Relaxed),
                fail: r.fail.load(Ordering::Relaxed),
                avg_time_ms: r.avg_time_ms(),
                max_time_ms: r.max_time_ms.load(Ordering::Relaxed),
            })
            .collect();
        out.sort_by(|a, b| b.total.cmp(&a.total));
        out
    }

    pub fn clear(&self) {
        self.stats.clear();
    }
}

/// 统计快照（对应 CompStatistics 的输出形态）
#[derive(Debug, Clone)]
pub struct CompStatistics {
    pub node_id: String,
    pub total: u64,
    pub success: u64,
    pub fail: u64,
    pub avg_time_ms: u64,
    pub max_time_ms: u64,
}

impl std::fmt::Display for CompStatistics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: total={}, success={}, fail={}, avg={}ms, max={}ms",
            self.node_id, self.total, self.success, self.fail, self.avg_time_ms, self.max_time_ms
        )
    }
}
