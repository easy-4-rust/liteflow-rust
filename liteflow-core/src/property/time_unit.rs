use serde::{Deserialize, Serialize};

/// Java `TimeUnit` 在 LiteFlow 配置中的可序列化映射。
///
/// 该枚举只承载 `whenMaxWaitTimeUnit` 配置，执行层可通过 `to_duration` 得到
/// Rust `Duration`。对应 Java: `java.util.concurrent.TimeUnit`。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimeUnit {
    /// 纳秒。
    Nanoseconds,
    /// 微秒。
    Microseconds,
    /// 毫秒，Java 默认值。
    #[default]
    Milliseconds,
    /// 秒。
    Seconds,
    /// 分钟。
    Minutes,
    /// 小时。
    Hours,
    /// 天。
    Days,
}

impl TimeUnit {
    /// 把数值和单位转换为 Rust 时长；溢出时使用饱和乘法避免回绕。
    ///
    /// 参数 `value` 为当前枚举单位下的时长数值，返回对应的 Rust `Duration`。
    #[must_use]
    pub fn to_duration(self, value: u64) -> std::time::Duration {
        match self {
            Self::Nanoseconds => std::time::Duration::from_nanos(value),
            Self::Microseconds => std::time::Duration::from_micros(value),
            Self::Milliseconds => std::time::Duration::from_millis(value),
            Self::Seconds => std::time::Duration::from_secs(value),
            Self::Minutes => std::time::Duration::from_secs(value.saturating_mul(60)),
            Self::Hours => std::time::Duration::from_secs(value.saturating_mul(60 * 60)),
            Self::Days => std::time::Duration::from_secs(value.saturating_mul(24 * 60 * 60)),
        }
    }
}
