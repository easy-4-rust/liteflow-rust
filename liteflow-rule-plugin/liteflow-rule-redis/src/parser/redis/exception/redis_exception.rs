//! Redis 规则插件异常。

/// Redis 配置、连接或规则刷新错误。
///
/// 对应 Java: `com.yomahub.liteflow.parser.redis.exception.RedisException`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisException {
    message: String,
}

impl RedisException {
    /// 使用错误消息创建异常。
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for RedisException {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for RedisException {}
