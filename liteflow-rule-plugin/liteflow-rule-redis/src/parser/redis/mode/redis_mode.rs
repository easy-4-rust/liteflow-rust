//! Redis 部署模式。

use serde::{Deserialize, Serialize};

/// Redis 单点、哨兵或集群模式。
///
/// 对应 Java: `com.yomahub.liteflow.parser.redis.mode.RedisMode`。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum RedisMode {
    /// 单点模式。
    #[default]
    Single,
    /// Sentinel 哨兵模式。
    Sentinel,
    /// Cluster 集群模式。
    Cluster,
}

impl RedisMode {
    /// 返回配置文件使用的小写模式名。对应 Java `RedisMode#getMode`。
    #[must_use]
    pub const fn get_mode(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Sentinel => "sentinel",
            Self::Cluster => "cluster",
        }
    }
}

impl From<String> for RedisMode {
    fn from(value: String) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "sentinel" => Self::Sentinel,
            "cluster" => Self::Cluster,
            _ => Self::Single,
        }
    }
}

impl From<RedisMode> for String {
    fn from(value: RedisMode) -> Self {
        value.get_mode().to_string()
    }
}
