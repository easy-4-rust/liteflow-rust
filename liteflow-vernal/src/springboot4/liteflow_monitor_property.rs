use serde::{Deserialize, Serialize};

/// LiteFlow 监控器的 Spring Boot 4 配置属性。
///
/// serde 对应 `@ConfigurationProperties(prefix = "liteflow.monitor")`，并忽略
/// 未知字段。对应 Java:
/// `com.yomahub.liteflow.springboot4.LiteflowMonitorProperty`。
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
    /// 返回是否打印监控日志。对应 Java: `isEnableLog`。
    #[must_use]
    pub fn is_enable_log(&self) -> bool {
        self.enable_log
    }

    /// 设置是否打印监控日志。参数 `enable_log` 为新的开关。
    pub fn set_enable_log(&mut self, enable_log: bool) {
        self.enable_log = enable_log;
    }

    /// 返回监控队列最大容量。对应 Java: `getQueueLimit`。
    #[must_use]
    pub fn get_queue_limit(&self) -> usize {
        self.queue_limit
    }

    /// 设置监控队列最大容量。参数 `queue_limit` 为新的容量。
    pub fn set_queue_limit(&mut self, queue_limit: usize) {
        self.queue_limit = queue_limit;
    }

    /// 返回首次打印前的延迟毫秒数。对应 Java: `getDelay`。
    #[must_use]
    pub fn get_delay(&self) -> u64 {
        self.delay
    }

    /// 设置首次打印前的延迟毫秒数。参数 `delay` 为新的延迟。
    pub fn set_delay(&mut self, delay: u64) {
        self.delay = delay;
    }

    /// 返回监控日志打印周期毫秒数。对应 Java: `getPeriod`。
    #[must_use]
    pub fn get_period(&self) -> u64 {
        self.period
    }

    /// 设置监控日志打印周期毫秒数。参数 `period` 为新的周期。
    pub fn set_period(&mut self, period: u64) {
        self.period = period;
    }
}
