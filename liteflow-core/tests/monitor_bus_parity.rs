//! Java `MonitorBus` 配置、统计快照和调度器生命周期回归测试。

use std::sync::Arc;
use std::time::Duration;

use liteflow_core::property::LiteflowConfig;
use liteflow_core::{CompStatistics, MonitorBus, MonitorTimeTask};

#[tokio::test]
async fn java_named_api_controls_real_statistics_and_scheduler_state() {
    let monitor_bus = Arc::new(MonitorBus::new());
    let mut config = LiteflowConfig::default();
    config.set_queue_limit(2);
    config.set_delay(1);
    config.set_period(5);
    config.set_enable_log(true);
    monitor_bus.set_liteflow_config(config.clone());

    assert_eq!(monitor_bus.get_liteflow_config(), config);
    monitor_bus.add_statistics(CompStatistics::new("OrderComponent", 5));
    monitor_bus.add_statistics(CompStatistics::new("OrderComponent", 3));
    monitor_bus.add_statistics(CompStatistics::new("OrderComponent", 1));
    let statistics = monitor_bus.get_statistics_map();
    assert_eq!(statistics["OrderComponent"].len(), 2);

    let handle = Arc::new(MonitorTimeTask::with_sink(Arc::clone(&monitor_bus), |_| {}))
        .spawn(Duration::from_secs(60), Duration::from_secs(60));
    monitor_bus.register_scheduler(handle.abort_handle());
    assert!(!handle.is_finished());

    monitor_bus.close_scheduler();
    let join_error = handle
        .await
        .expect_err("closeScheduler 应取消真实 Tokio 调度任务");
    assert!(join_error.is_cancelled());

    // Java shutdown 可重复调用；Rust 对等入口也必须保持幂等。
    monitor_bus.close_scheduler();
}
