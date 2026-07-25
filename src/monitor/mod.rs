//! 对应 monitor 包：MonitorBus 统计（CompStatistics 语义）。
//! 按组件聚合执行次数、成功/失败、耗时，支持定时报表。

pub mod monitor_bus;

pub use monitor_bus::{CompStatistics, MonitorBus};
