//! Redis 规则监听模式。

use serde::{Deserialize, Serialize};

/// Redis 规则采用轮询或订阅方式刷新。
///
/// 对应 Java: `com.yomahub.liteflow.parser.redis.mode.RedisParserMode`。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum RedisParserMode {
    /// 周期轮询。
    #[default]
    Poll,
    /// 订阅模式的短名称。
    Sub,
    /// 订阅模式的完整名称。
    Subscribe,
}

impl RedisParserMode {
    /// 返回 Java 配置使用的模式名。对应 Java `RedisParserMode#getMode`。
    #[must_use]
    pub const fn get_mode(self) -> &'static str {
        match self {
            Self::Poll => "poll",
            Self::Sub | Self::Subscribe => "subscribe",
        }
    }

    /// 返回当前是否为订阅模式。
    #[must_use]
    pub const fn is_subscribe(self) -> bool {
        matches!(self, Self::Sub | Self::Subscribe)
    }
}

impl From<String> for RedisParserMode {
    fn from(value: String) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "sub" => Self::Sub,
            "subscribe" => Self::Subscribe,
            _ => Self::Poll,
        }
    }
}

impl From<RedisParserMode> for String {
    fn from(value: RedisParserMode) -> Self {
        value.get_mode().to_string()
    }
}
