use serde::{Deserialize, Serialize};

/// LiteFlow 监控器的 Spring Boot 配置属性。
///
/// serde 接管 Java `@ConfigurationProperties(prefix = "liteflow.monitor")` 的绑定，
/// 未知字段保持忽略。对应 Java:
/// `com.yomahub.liteflow.springboot.LiteflowMonitorProperty`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LiteflowMonitorProperty {
    enable_log: bool,
    queue_limit: usize,
    delay: u64,
    period: u64,
}

impl Default for LiteflowMonitorProperty {
    fn default() -> Self {
        Self {
            enable_log: false,
            queue_limit: 200,
            delay: 300_000,
            period: 300_000,
        }
    }
}

impl LiteflowMonitorProperty {
    /// 返回是否打印监控日志。
    ///
    /// # 返回
    /// 开启返回 `true`。对应 Java: `LiteflowMonitorProperty#isEnableLog`。
    #[must_use]
    pub fn is_enable_log(&self) -> bool {
        self.enable_log
    }

    /// 设置是否打印监控日志。
    ///
    /// # 参数
    /// - `enable_log`：新的监控日志开关。
    pub fn set_enable_log(&mut self, enable_log: bool) {
        self.enable_log = enable_log;
    }

    /// 返回监控队列最大容量。
    ///
    /// # 返回
    /// 每个组件保留的统计条目上限。对应 Java:
    /// `LiteflowMonitorProperty#getQueueLimit`。
    #[must_use]
    pub fn get_queue_limit(&self) -> usize {
        self.queue_limit
    }

    /// 设置监控队列最大容量。
    ///
    /// # 参数
    /// - `queue_limit`：新的统计条目上限。
    pub fn set_queue_limit(&mut self, queue_limit: usize) {
        self.queue_limit = queue_limit;
    }

    /// 返回首次打印前的延迟毫秒数。
    ///
    /// 对应 Java: `LiteflowMonitorProperty#getDelay`。
    #[must_use]
    pub fn get_delay(&self) -> u64 {
        self.delay
    }

    /// 设置首次打印前的延迟毫秒数。
    ///
    /// # 参数
    /// - `delay`：新的延迟毫秒数。
    pub fn set_delay(&mut self, delay: u64) {
        self.delay = delay;
    }

    /// 返回监控日志打印周期毫秒数。
    ///
    /// 对应 Java: `LiteflowMonitorProperty#getPeriod`。
    #[must_use]
    pub fn get_period(&self) -> u64 {
        self.period
    }

    /// 设置监控日志打印周期毫秒数。
    ///
    /// # 参数
    /// - `period`：新的周期毫秒数。
    pub fn set_period(&mut self, period: u64) {
        self.period = period;
    }
}
