//! 对应 Java 类：com.yomahub.liteflow.monitor.MonitorBus
//!
//! 组件执行统计总线：按组件（节点）ID 聚合执行次数、成功/失败次数与耗时，
//! 支持生成统计报表（对应 Java 端 MonitorTimeTask 定时打印的统计数据来源）。
//!
//! 说明：Java 中实时聚合字段放在 CompStatistics 内（AtomicLong），Rust 侧将其
//! 内聚为本文件内的 StatEntry（MonitorBus 的私有聚合单元），对外快照见
//! comp_statistics.rs 的 CompStatistics。

use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use super::comp_statistics::CompStatistics;

/// 单个组件的实时统计聚合项（对应 Java CompStatistics 的 AtomicLong 聚合部分，
/// 为 MonitorBus 的内部聚合单元，不对外暴露快照语义）
#[derive(Debug, Default)]
pub struct StatEntry {
    pub total: AtomicU64,
    pub success: AtomicU64,
    pub fail: AtomicU64,
    pub total_time_ms: AtomicU64,
    pub max_time_ms: AtomicU64,
}

impl StatEntry {
    /// 平均耗时（毫秒），对应 Java CompStatistics 的平均耗时计算
    pub fn avg_time_ms(&self) -> u64 {
        let total = self.total.load(Ordering::Relaxed);
        if total == 0 {
            0
        } else {
            self.total_time_ms.load(Ordering::Relaxed) / total
        }
    }
}

/// 对应 MonitorBus：组件执行统计总线
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

    /// 统计报表（对应定时打印的统计信息，输出 CompStatistics 快照列表，
    /// 按总执行次数降序排列）
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

    /// 清空全部统计（对应 MonitorBus 的统计重置）
    pub fn clear(&self) {
        self.stats.clear();
    }
}
