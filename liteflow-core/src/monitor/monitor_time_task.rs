use std::sync::Arc;
use std::time::Duration;

use super::MonitorBus;

type ReportSink = dyn Fn(&str) + Send + Sync;

/// 周期输出 LiteFlow 监控统计的定时任务。
///
/// Java 继承 `TimerTask` 并由 `ScheduledExecutorService` 调度；Rust 使用
/// `tokio::spawn` 与周期定时器，返回 `JoinHandle` 供宿主取消和等待。
///
/// 对应 Java: `com.yomahub.liteflow.monitor.MonitorTimeTask`。
pub struct MonitorTimeTask {
    monitor_bus: Arc<MonitorBus>,
    report_sink: Arc<ReportSink>,
}

impl MonitorTimeTask {
    /// 创建使用标准错误输出作为日志后端的定时任务。
    ///
    /// 参数 `monitor_bus` 为统计数据源。对应 Java:
    /// `MonitorTimeTask#MonitorTimeTask(MonitorBus)`。
    #[must_use]
    pub fn new(monitor_bus: Arc<MonitorBus>) -> Self {
        Self::with_sink(monitor_bus, |report| eprintln!("{report}"))
    }

    /// 创建使用自定义报表接收器的定时任务。
    ///
    /// 该入口让 Vernal 或其他宿主接入自己的 tracing/logging 后端。
    #[must_use]
    pub fn with_sink(
        monitor_bus: Arc<MonitorBus>,
        report_sink: impl Fn(&str) + Send + Sync + 'static,
    ) -> Self {
        Self {
            monitor_bus,
            report_sink: Arc::new(report_sink),
        }
    }

    /// 立即生成并输出一次统计报表。
    ///
    /// 返回生成的文本，方便调用方进一步持久化。
    /// 对应 Java: `MonitorTimeTask#run`。
    pub fn run(&self) -> String {
        let report = self.monitor_bus.print_statistics();
        (self.report_sink)(&report);
        report
    }

    /// 按初始延迟和固定周期启动真实异步调度。
    ///
    /// 参数 `delay`、`period` 分别对应 Java 配置中的毫秒延迟与周期；零周期
    /// 会提升为 1 毫秒。取消返回的 `JoinHandle` 即等价于关闭调度器。
    pub fn spawn(
        self: Arc<Self>,
        delay: Duration,
        period: Duration,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            let mut interval = tokio::time::interval(period.max(Duration::from_millis(1)));
            loop {
                interval.tick().await;
                self.run();
            }
        })
    }
}
