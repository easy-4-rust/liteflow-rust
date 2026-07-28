//! MonitorBus 内部实时统计项。

use std::sync::atomic::{AtomicU64, Ordering};

/// 单个组件的实时统计聚合项。
///
/// 这是 Rust MonitorBus 的原子聚合伴随类型，承接 Java MonitorBus 内部统计
/// Map 的计数职责，不对应独立 Java 对象。
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
    ///
    /// 尚无样本时返回 0；否则返回总耗时除以样本数的整数结果。
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
