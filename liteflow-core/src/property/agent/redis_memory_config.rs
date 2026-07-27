use serde::{Deserialize, Serialize};

/// Redis 记忆后端支持的客户端类型。
///
/// 这是 Java `RedisMemoryConfig` 的内部枚举，按规则与主对象保留在同一文件。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RedisClientType {
    /// Redisson 客户端。
    #[default]
    Redisson,
    /// Jedis 客户端。
    Jedis,
    /// Lettuce 客户端。
    Lettuce,
}

/// Redis 记忆后端配置。
///
/// Redis 客户端由宿主创建并注册，配置通过名称和客户端类型定位，不在 LiteFlow
/// 内部建立连接。
///
/// 对应 Java: `com.yomahub.liteflow.property.agent.RedisMemoryConfig`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RedisMemoryConfig {
    /// Redis 客户端 Bean 名称。
    pub bean_name: Option<String>,
    /// 已注册客户端的类型。
    pub client_type: RedisClientType,
    /// 会话 key 前缀，用于环境和业务隔离。
    pub key_prefix: String,
}

impl Default for RedisMemoryConfig {
    fn default() -> Self {
        Self {
            bean_name: None,
            client_type: RedisClientType::Redisson,
            key_prefix: "liteflow:agent:session".to_string(),
        }
    }
}

impl RedisMemoryConfig {
    /// 返回 Redis 客户端 Bean 名称。对应 Java: `RedisMemoryConfig#getBeanName`。
    #[must_use]
    pub fn bean_name(&self) -> Option<&str> {
        self.bean_name.as_deref()
    }

    /// 设置 Redis 客户端 Bean 名称。对应 Java: `RedisMemoryConfig#setBeanName`。
    pub fn set_bean_name(&mut self, bean_name: Option<String>) {
        self.bean_name = bean_name;
    }

    /// 返回客户端类型。对应 Java: `RedisMemoryConfig#getClientType`。
    #[must_use]
    pub fn client_type(&self) -> RedisClientType {
        self.client_type
    }

    /// 设置客户端类型。对应 Java: `RedisMemoryConfig#setClientType`。
    pub fn set_client_type(&mut self, client_type: RedisClientType) {
        self.client_type = client_type;
    }

    /// 返回会话 key 前缀。对应 Java: `RedisMemoryConfig#getKeyPrefix`。
    #[must_use]
    pub fn key_prefix(&self) -> &str {
        &self.key_prefix
    }

    /// 设置会话 key 前缀。对应 Java: `RedisMemoryConfig#setKeyPrefix`。
    pub fn set_key_prefix(&mut self, key_prefix: impl Into<String>) {
        self.key_prefix = key_prefix.into();
    }
}
