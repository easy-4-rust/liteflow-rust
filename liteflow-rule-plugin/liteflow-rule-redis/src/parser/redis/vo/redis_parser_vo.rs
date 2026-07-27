//! Redis 规则扩展配置对象。

use serde::{Deserialize, Deserializer, Serialize};

use crate::parser::redis::mode::{RedisMode, RedisParserMode};

/// Redis 连接、监听模式及 Chain/Script Hash 配置。
///
/// 对应 Java: `com.yomahub.liteflow.parser.redis.vo.RedisParserVO`。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RedisParserVO {
    /// Redis 单点、哨兵或集群模式。
    pub redis_mode: RedisMode,
    /// 单点主机。
    pub host: Option<String>,
    /// 单点端口。
    pub port: Option<u16>,
    /// Sentinel 主节点名。
    pub master_name: Option<String>,
    /// Sentinel 地址列表，也接受逗号分隔字符串。
    #[serde(deserialize_with = "deserialize_addresses")]
    pub sentinel_address: Vec<String>,
    /// Redis 6+ 用户名。
    pub username: Option<String>,
    /// Redis 密码。
    pub password: Option<String>,
    /// 最小空闲连接数。
    pub connection_minimum_idle_size: usize,
    /// 连接池容量。
    pub connection_pool_size: usize,
    /// 轮询或订阅模式。
    pub mode: RedisParserMode,
    /// 轮询间隔秒数。
    pub polling_interval: u64,
    /// 首次轮询延迟秒数。
    pub polling_start_time: u64,
    /// Chain 所在数据库。
    pub chain_data_base: Option<i64>,
    /// Chain Hash key。
    pub chain_key: Option<String>,
    /// Script 所在数据库。
    pub script_data_base: Option<i64>,
    /// Script Hash key。
    pub script_key: Option<String>,
    /// Cluster 节点列表，也兼容 Java `clusterAddress`。
    #[serde(alias = "clusterAddress", deserialize_with = "deserialize_addresses")]
    pub cluster_node_address: Vec<String>,
}

impl Default for RedisParserVO {
    fn default() -> Self {
        Self {
            redis_mode: RedisMode::Single,
            host: None,
            port: None,
            master_name: None,
            sentinel_address: Vec::new(),
            username: None,
            password: None,
            connection_minimum_idle_size: 2,
            connection_pool_size: 4,
            mode: RedisParserMode::Poll,
            polling_interval: 60,
            polling_start_time: 60,
            chain_data_base: None,
            chain_key: None,
            script_data_base: None,
            script_key: None,
            cluster_node_address: Vec::new(),
        }
    }
}

impl RedisParserVO {
    /// 校验连接模式和 Chain/Script 数据库配置。
    ///
    /// 对应 Java `RedisXmlELParser#checkParserVO`。
    pub fn validate(&self) -> Result<(), String> {
        match self.redis_mode {
            RedisMode::Single if is_blank(self.host.as_deref()) => {
                return Err("ruleSourceExtData host is blank".to_string());
            }
            RedisMode::Single if self.port.is_none() => {
                return Err("ruleSourceExtData port is blank".to_string());
            }
            RedisMode::Sentinel if is_blank(self.master_name.as_deref()) => {
                return Err("ruleSourceExtData master name is blank".to_string());
            }
            RedisMode::Sentinel if self.sentinel_address.is_empty() => {
                return Err("ruleSourceExtData sentinel address list is blank".to_string());
            }
            RedisMode::Cluster if self.cluster_node_address.is_empty() => {
                return Err("ruleSourceExtData cluster address list is blank".to_string());
            }
            _ => {}
        }
        if self.chain_data_base.is_none() && self.redis_mode != RedisMode::Cluster {
            return Err("ruleSourceExtData chainDataBase is blank".to_string());
        }
        if is_blank(self.chain_key.as_deref()) {
            return Err("ruleSourceExtData chainKey is blank".to_string());
        }
        if self.script_key.is_some()
            && self.script_data_base.is_none()
            && self.redis_mode != RedisMode::Cluster
        {
            return Err("ruleSourceExtData scriptDataBase is blank".to_string());
        }
        Ok(())
    }
}

fn is_blank(value: Option<&str>) -> bool {
    value.is_none_or(|value| value.trim().is_empty())
}

fn deserialize_addresses<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Null => Ok(Vec::new()),
        serde_json::Value::String(addresses) => Ok(addresses
            .split(',')
            .map(str::trim)
            .filter(|address| !address.is_empty())
            .map(str::to_string)
            .collect()),
        serde_json::Value::Array(values) => values
            .into_iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_string)
                    .ok_or_else(|| serde::de::Error::custom("redis address must be a string"))
            })
            .collect(),
        _ => Err(serde::de::Error::custom(
            "redis addresses must be a string or string array",
        )),
    }
}
