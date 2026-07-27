//! Redis 解析模式公共契约。

use liteflow_core::exception::{LFResult, LiteflowError};

use super::{RClient, RedisMode};
use crate::parser::redis::vo::RedisParserVO;

/// Redis 规则内容获取和监听的公共接口。
///
/// 对应 Java: `com.yomahub.liteflow.parser.redis.mode.RedisParserHelper`。
pub trait RedisParserHelper: Send + Sync {
    /// 获取可交给 XML EL Parser 的完整规则文本。
    ///
    /// 对应 Java `RedisParserHelper#getContent`。
    fn get_content(&self) -> LFResult<String>;

    /// 创建当前拓扑下指定数据库的 Redis 客户端。
    ///
    /// 参数 `redis_parser_vo`、`data_base` 分别对应 Java
    /// `RedisParserHelper#getSingleRedissonConfig` /
    /// `getSentinelRedissonConfig` 的同名参数；Cluster 按 Redis 规范固定使用数据库 0。
    fn get_redis_client(
        redis_parser_vo: &RedisParserVO,
        data_base: Option<i64>,
    ) -> LFResult<Option<RClient>>
    where
        Self: Sized,
    {
        match redis_parser_vo.redis_mode {
            RedisMode::Single => Self::get_single_redis_config(redis_parser_vo, data_base),
            RedisMode::Sentinel => Self::get_sentinel_redis_config(redis_parser_vo, data_base),
            RedisMode::Cluster => Self::get_cluster_redis_config(redis_parser_vo).map(Some),
        }
    }

    /// 构造单点模式客户端；数据库号缺失时与 Java 一样返回空。
    ///
    /// 对应 Java `RedisParserHelper#getSingleRedissonConfig`。
    fn get_single_redis_config(
        redis_parser_vo: &RedisParserVO,
        data_base: Option<i64>,
    ) -> LFResult<Option<RClient>>
    where
        Self: Sized,
    {
        let Some(data_base) = data_base else {
            return Ok(None);
        };
        let host = redis_parser_vo
            .host
            .clone()
            .ok_or_else(|| LiteflowError::Rule("redis host is blank".to_string()))?;
        let port = redis_parser_vo
            .port
            .ok_or_else(|| LiteflowError::Rule("redis port is blank".to_string()))?;
        RClient::single(
            host,
            port,
            data_base,
            redis_parser_vo.username.clone(),
            redis_parser_vo.password.clone(),
        )
        .map(Some)
    }

    /// 构造 Sentinel 主节点客户端；数据库号缺失时与 Java 一样返回空。
    ///
    /// 对应 Java `RedisParserHelper#getSentinelRedissonConfig`。
    fn get_sentinel_redis_config(
        redis_parser_vo: &RedisParserVO,
        data_base: Option<i64>,
    ) -> LFResult<Option<RClient>>
    where
        Self: Sized,
    {
        let Some(data_base) = data_base else {
            return Ok(None);
        };
        let master_name = redis_parser_vo
            .master_name
            .clone()
            .ok_or_else(|| LiteflowError::Rule("redis master name is blank".to_string()))?;
        RClient::sentinel(
            &redis_parser_vo.sentinel_address,
            master_name,
            data_base,
            redis_parser_vo.username.clone(),
            redis_parser_vo.password.clone(),
        )
        .map(Some)
    }

    /// 构造 Cluster 客户端。
    ///
    /// 对应 Java `RedisParserHelper#getCluserRedissonConfig`，方法名中的 `Cluser`
    /// 是 Java 源码既有拼写；Rust API 使用正确的 `cluster`。
    fn get_cluster_redis_config(redis_parser_vo: &RedisParserVO) -> LFResult<RClient>
    where
        Self: Sized,
    {
        RClient::cluster(
            &redis_parser_vo.cluster_node_address,
            redis_parser_vo.username.clone(),
            redis_parser_vo.password.clone(),
        )
    }
}
