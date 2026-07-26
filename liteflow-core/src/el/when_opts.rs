//! WHEN 条件选项。

/// WHEN 的执行选项，对应 Java `WhenCondition` 字段。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WhenOpts {
    /// 任一分支完成即返回。
    pub any: bool,
    /// 忽略分支错误。
    pub ignore_error: bool,
    /// 成功百分比阈值。
    pub percentage: Option<f64>,
    /// 必须成功的节点 id。
    pub must: Vec<String>,
    /// 最大等待毫秒数。
    pub max_wait_ms: Option<u64>,
    /// Java 线程池名；Rust 端保留为调度元数据。
    pub thread_pool: Option<String>,
}
