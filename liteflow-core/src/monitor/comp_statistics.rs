//! 对应 Java 类：com.yomahub.liteflow.monitor.CompStatistics
//!
//! 组件统计快照对象。Java 中 CompStatistics 既承担实时聚合（AtomicLong 累加），
//! 也承担报表输出；Rust 侧按"一文件一对象"拆分后，实时聚合由 monitor_bus.rs 中的
//! 内部 StatEntry 承担，本对象仅作为某一时刻的统计快照（对应 Java 端定时报表时
//! 打印的 CompStatistics 形态）。

/// 统计快照（对应 CompStatistics 的输出形态）
#[derive(Debug, Clone)]
pub struct CompStatistics {
    /// 组件（节点）ID
    pub node_id: String,
    /// 总执行次数
    pub total: u64,
    /// 成功次数
    pub success: u64,
    /// 失败次数
    pub fail: u64,
    /// 平均耗时（毫秒）
    pub avg_time_ms: u64,
    /// 最大耗时（毫秒）
    pub max_time_ms: u64,
}

impl std::fmt::Display for CompStatistics {
    /// 对应 Java CompStatistics 的统计信息打印格式
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: total={}, success={}, fail={}, avg={}ms, max={}ms",
            self.node_id, self.total, self.success, self.fail, self.avg_time_ms, self.max_time_ms
        )
    }
}
