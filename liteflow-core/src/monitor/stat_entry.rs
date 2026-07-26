//! MonitorBus 内部实时统计项。

use std::sync::atomic::{AtomicU64, Ordering};

/// 单个组件的实时统计聚合项。
#[derive(Debug, Default)]
pub struct StatEntry {
    pub total: AtomicU64,
    pub success: AtomicU64,
    pub fail: AtomicU64,
    pub total_time_ms: AtomicU64,
    pub max_time_ms: AtomicU64,
}

impl StatEntry {
    /// 返回平均耗时毫秒数。
    #[must_use]
    pub fn avg_time_ms(&self) -> u64 {
        let total = self.total.load(Ordering::Relaxed);
        if total == 0 {
            0
        } else {
            self.total_time_ms.load(Ordering::Relaxed) / total
        }
    }
}
