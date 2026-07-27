//! Redis 客户端封装。

use std::collections::{HashMap, HashSet};

use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::rule_plugin::fnv_fp;
use redis::cluster::ClusterClient;
use redis::sentinel::{Sentinel, SentinelClient, SentinelNodeConnectionInfo, SentinelServerType};
use redis::{
    Client, ConnectionAddr, ConnectionInfo, ConnectionLike, FromRedisValue, IntoConnectionInfo,
    ProtocolVersion, RedisConnectionInfo,
};

/// Redis 连接拓扑。
///
/// 这是 `RClient` 的 Rust 内部实现细节，用于承接 Redisson 单点、哨兵和集群
/// `Config` 的差异；它不对应额外的 Java 对象。
#[derive(Clone)]
enum RedisConnectionTarget {
    SingleUrl(String),
    Single(ConnectionInfo),
    Sentinel {
        sentinel_urls: Vec<String>,
        master_name: String,
        node_connection_info: SentinelNodeConnectionInfo,
    },
    Cluster {
        node_urls: Vec<String>,
        username: Option<String>,
        password: Option<String>,
    },
}

/// 对 Redis 单点、Sentinel 和 Cluster 命令的统一封装。
///
/// 对应 Java: `com.yomahub.liteflow.parser.redis.mode.RClient`。
#[derive(Clone)]
pub struct RClient {
    target: RedisConnectionTarget,
}

impl std::fmt::Debug for RClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 诊断信息只暴露拓扑种类，避免 URL、用户名和密码进入日志。
        let topology = match &self.target {
            RedisConnectionTarget::SingleUrl(_) | RedisConnectionTarget::Single(_) => "single",
            RedisConnectionTarget::Sentinel { .. } => "sentinel",
            RedisConnectionTarget::Cluster { .. } => "cluster",
        };
        formatter
            .debug_struct("RClient")
            .field("topology", &topology)
            .finish()
    }
}

impl RClient {
    /// 使用完整 Redis URL 创建单点客户端。
    ///
    /// 保留早期 Rust API；规则扩展配置应优先通过
    /// `RedisParserHelper#getSingleRedissonConfig` 对应方法创建。
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            target: RedisConnectionTarget::SingleUrl(url.into()),
        }
    }

    /// 使用单点地址、认证信息和数据库号创建客户端。
    ///
    /// 对应 Java `RedisParserHelper#getSingleRedissonConfig` 构造出的 Redisson 客户端。
    pub fn single(
        host: impl Into<String>,
        port: u16,
        database: i64,
        username: Option<String>,
        password: Option<String>,
    ) -> LFResult<Self> {
        let host = host.into();
        if host.trim().is_empty() {
            return Err(LiteflowError::Rule("redis host is blank".to_string()));
        }
        Ok(Self {
            target: RedisConnectionTarget::Single(ConnectionInfo {
                addr: ConnectionAddr::Tcp(host, port),
                redis: redis_connection_info(database, username, password),
            }),
        })
    }

    /// 使用 Sentinel 地址与主节点名创建始终连接主节点的客户端。
    ///
    /// 每次命令执行前都会重新向 Sentinel 解析主节点，因此主从切换后下一次操作
    /// 能重新定位新主节点。对应 Java `RedisParserHelper#getSentinelRedissonConfig`。
    pub fn sentinel(
        sentinel_addresses: &[String],
        master_name: impl Into<String>,
        database: i64,
        username: Option<String>,
        password: Option<String>,
    ) -> LFResult<Self> {
        let sentinel_urls = redis_urls(sentinel_addresses, "sentinel")?;
        let master_name = master_name.into();
        if master_name.trim().is_empty() {
            return Err(LiteflowError::Rule(
                "redis sentinel master name is blank".to_string(),
            ));
        }
        let node_connection_info = SentinelNodeConnectionInfo {
            tls_mode: None,
            redis_connection_info: Some(redis_connection_info(database, username, password)),
        };

        // 构造阶段先复用 redis crate 的 URL 校验，网络连接延迟到实际命令执行。
        SentinelClient::build(
            sentinel_urls.clone(),
            master_name.clone(),
            Some(node_connection_info.clone()),
            SentinelServerType::Master,
        )
        .map_err(|error| redis_error("sentinel config", error))?;
        Ok(Self {
            target: RedisConnectionTarget::Sentinel {
                sentinel_urls,
                master_name,
                node_connection_info,
            },
        })
    }

    /// 使用 Redis Cluster 初始节点和统一认证信息创建客户端。
    ///
    /// Cluster 不选择逻辑数据库，数据始终位于数据库 0。对应 Java
    /// `RedisParserHelper#getCluserRedissonConfig`（保留 Java 原方法拼写来源）。
    pub fn cluster(
        cluster_node_addresses: &[String],
        username: Option<String>,
        password: Option<String>,
    ) -> LFResult<Self> {
        let node_urls = redis_urls(cluster_node_addresses, "cluster")?;
        cluster_client(&node_urls, username.clone(), password.clone())
            .map_err(|error| redis_error("cluster config", error))?;
        Ok(Self {
            target: RedisConnectionTarget::Cluster {
                node_urls,
                username,
                password,
            },
        })
    }

    /// 读取 Hash 全部字段。
    ///
    /// 参数 `key` 为 Hash 名称；返回字段到值的完整映射。对应 Java `RClient#getMap`。
    pub fn get_map(&self, key: &str) -> LFResult<HashMap<String, String>> {
        self.query(redis::cmd("HGETALL").arg(key), "hgetall")
    }

    /// 对 Hash 的字段和值生成稳定指纹，字段顺序不影响结果。
    ///
    /// Rust 以一次 `HGETALL` 替代 Java polling task 中逐字段执行 Lua SHA。
    pub fn hash_fingerprint(&self, key: &str) -> LFResult<String> {
        let mut entries: Vec<_> = self.get_map(key)?.into_iter().collect();
        entries.sort_by(|left, right| left.0.cmp(&right.0));
        let content = entries
            .into_iter()
            .map(|(field, value)| format!("{field}\0{value}\0"))
            .collect::<String>();
        Ok(fnv_fp(&content))
    }

    /// 读取 Hash 全部字段名。
    ///
    /// 参数 `key` 为 Hash 名称；返回当前全部字段。对应 Java `RClient#hkeys`。
    pub fn hkeys(&self, key: &str) -> LFResult<HashSet<String>> {
        self.query(redis::cmd("HKEYS").arg(key), "hkeys")
    }

    /// 读取指定 Hash 字段值。
    ///
    /// 参数 `key` 为 Hash 名称，`field` 为字段；字段不存在时返回 `None`。
    /// 对应 Java `RClient#hget`。
    pub fn hget(&self, key: &str, field: &str) -> LFResult<Option<String>> {
        self.query(redis::cmd("HGET").arg(key).arg(field), "hget")
    }

    /// 装载 Lua 脚本并返回 SHA-1 摘要。
    ///
    /// 参数 `lua_script` 为脚本文本。对应 Java `RClient#scriptLoad`。
    pub fn script_load(&self, lua_script: &str) -> LFResult<String> {
        self.query(
            redis::cmd("SCRIPT").arg("LOAD").arg(lua_script),
            "script load",
        )
    }

    /// 按 SHA 执行缓存中的 Lua 脚本。
    ///
    /// 参数 `sha_digest` 为脚本摘要，`args` 与 Java 可变参数语义一致，同时作为
    /// Redis key 列表参与 Cluster 槽位路由。对应 Java `RClient#evalSha`。
    pub fn eval_sha(&self, sha_digest: &str, args: &[&str]) -> LFResult<Option<String>> {
        let mut command = redis::cmd("EVALSHA");
        command.arg(sha_digest).arg(args.len());
        for argument in args {
            command.arg(argument);
        }
        self.query(&command, "evalsha")
    }

    /// 返回需要建立 keyspace 订阅的 Redis 节点客户端。
    ///
    /// 单点返回目标节点，Sentinel 在调用时解析当前主节点，Cluster 通过
    /// `CLUSTER NODES` 发现全部健康主节点。对应 Java `RClient#addListener`
    /// 由 Redisson 隐式完成的拓扑选择部分。
    pub(crate) fn pubsub_clients(&self) -> LFResult<Vec<Client>> {
        match &self.target {
            RedisConnectionTarget::SingleUrl(url) => Client::open(url.as_str())
                .map(|client| vec![client])
                .map_err(|error| redis_error("pubsub open", error)),
            RedisConnectionTarget::Single(connection_info) => Client::open(connection_info.clone())
                .map(|client| vec![client])
                .map_err(|error| redis_error("pubsub open", error)),
            RedisConnectionTarget::Sentinel {
                sentinel_urls,
                master_name,
                node_connection_info,
            } => {
                let mut sentinel = Sentinel::build(sentinel_urls.clone())
                    .map_err(|error| redis_error("sentinel config", error))?;
                sentinel
                    .master_for(master_name, Some(node_connection_info))
                    .map(|client| vec![client])
                    .map_err(|error| redis_error("sentinel master resolve", error))
            }
            RedisConnectionTarget::Cluster {
                node_urls,
                username,
                password,
            } => cluster_master_clients(node_urls, username.clone(), password.clone()),
        }
    }

    fn query<T>(&self, command: &redis::Cmd, operation: &str) -> LFResult<T>
    where
        T: FromRedisValue,
    {
        let mut connection = self.connection()?;
        command
            .query(connection.as_mut())
            .map_err(|error| redis_error(operation, error))
    }

    fn connection(&self) -> LFResult<Box<dyn ConnectionLike>> {
        match &self.target {
            RedisConnectionTarget::SingleUrl(url) => redis::Client::open(url.as_str())
                .map_err(|error| redis_error("open", error))?
                .get_connection()
                .map(|connection| Box::new(connection) as Box<dyn ConnectionLike>)
                .map_err(|error| redis_error("connect", error)),
            RedisConnectionTarget::Single(connection_info) => {
                redis::Client::open(connection_info.clone())
                    .map_err(|error| redis_error("open", error))?
                    .get_connection()
                    .map(|connection| Box::new(connection) as Box<dyn ConnectionLike>)
                    .map_err(|error| redis_error("connect", error))
            }
            RedisConnectionTarget::Sentinel {
                sentinel_urls,
                master_name,
                node_connection_info,
            } => {
                let mut client = SentinelClient::build(
                    sentinel_urls.clone(),
                    master_name.clone(),
                    Some(node_connection_info.clone()),
                    SentinelServerType::Master,
                )
                .map_err(|error| redis_error("sentinel config", error))?;
                client
                    .get_connection()
                    .map(|connection| Box::new(connection) as Box<dyn ConnectionLike>)
                    .map_err(|error| redis_error("sentinel connect", error))
            }
            RedisConnectionTarget::Cluster {
                node_urls,
                username,
                password,
            } => cluster_client(node_urls, username.clone(), password.clone())
                .map_err(|error| redis_error("cluster config", error))?
                .get_connection()
                .map(|connection| Box::new(connection) as Box<dyn ConnectionLike>)
                .map_err(|error| redis_error("cluster connect", error)),
        }
    }
}

fn redis_connection_info(
    database: i64,
    username: Option<String>,
    password: Option<String>,
) -> RedisConnectionInfo {
    let (username, password) = normalized_credentials(username, password);
    RedisConnectionInfo {
        db: database,
        username,
        password,
        protocol: ProtocolVersion::RESP2,
    }
}

fn redis_urls(addresses: &[String], topology: &str) -> LFResult<Vec<String>> {
    let urls = addresses
        .iter()
        .map(|address| address.trim())
        .filter(|address| !address.is_empty())
        .map(|address| {
            if address.starts_with("redis://") || address.starts_with("rediss://") {
                address.to_string()
            } else {
                format!("redis://{address}")
            }
        })
        .collect::<Vec<_>>();
    if urls.is_empty() {
        return Err(LiteflowError::Rule(format!(
            "redis {topology} address list is blank"
        )));
    }
    Ok(urls)
}

fn cluster_client(
    node_urls: &[String],
    username: Option<String>,
    password: Option<String>,
) -> redis::RedisResult<ClusterClient> {
    let (username, password) = normalized_credentials(username, password);
    let mut builder = ClusterClient::builder(node_urls.to_vec());
    if let Some(username) = username {
        builder = builder.username(username);
    }
    if let Some(password) = password {
        builder = builder.password(password);
    }
    builder.build()
}

fn normalized_credentials(
    username: Option<String>,
    password: Option<String>,
) -> (Option<String>, Option<String>) {
    let username = username.filter(|value| !value.trim().is_empty());
    let password = password.filter(|value| !value.trim().is_empty());
    match (username, password) {
        (Some(username), Some(password)) => (Some(username), Some(password)),
        (_, Some(password)) => (None, Some(password)),
        _ => (None, None),
    }
}

fn cluster_master_clients(
    node_urls: &[String],
    username: Option<String>,
    password: Option<String>,
) -> LFResult<Vec<Client>> {
    if node_urls.is_empty() {
        return Err(LiteflowError::Rule(
            "redis cluster address list is blank".to_string(),
        ));
    }
    let mut nodes = None;
    let mut last_error = None;
    for seed_url in node_urls {
        let result = (|| {
            let seed_info =
                authenticated_connection_info(seed_url, 0, username.clone(), password.clone())?;
            let seed_client =
                Client::open(seed_info).map_err(|error| redis_error("cluster seed open", error))?;
            let mut connection = seed_client
                .get_connection()
                .map_err(|error| redis_error("cluster seed connect", error))?;
            redis::cmd("CLUSTER")
                .arg("NODES")
                .query(&mut connection)
                .map_err(|error| redis_error("cluster nodes", error))
        })();
        match result {
            Ok(value) => {
                nodes = Some(value);
                break;
            }
            Err(error) => last_error = Some(error),
        }
    }
    let nodes: String = nodes.ok_or_else(|| {
        last_error.unwrap_or_else(|| {
            LiteflowError::Rule("redis cluster has no reachable seed nodes".to_string())
        })
    })?;

    let mut clients = Vec::new();
    for line in nodes.lines() {
        let mut fields = line.split_whitespace();
        let _node_id = fields.next();
        let Some(raw_address) = fields.next() else {
            continue;
        };
        let Some(flags) = fields.next() else {
            continue;
        };
        let flags = flags.split(',').collect::<HashSet<_>>();
        if !flags.contains("master")
            || flags.contains("fail")
            || flags.contains("fail?")
            || flags.contains("handshake")
            || flags.contains("noaddr")
        {
            continue;
        }
        let address = raw_address
            .split('@')
            .next()
            .unwrap_or(raw_address)
            .split(',')
            .next()
            .unwrap_or(raw_address);
        let url = if address.starts_with("redis://") || address.starts_with("rediss://") {
            address.to_string()
        } else {
            format!("redis://{address}")
        };
        let connection_info =
            authenticated_connection_info(&url, 0, username.clone(), password.clone())?;
        clients.push(
            Client::open(connection_info)
                .map_err(|error| redis_error("cluster pubsub node open", error))?,
        );
    }
    if clients.is_empty() {
        return Err(LiteflowError::Rule(
            "redis cluster has no healthy master nodes".to_string(),
        ));
    }
    Ok(clients)
}

fn authenticated_connection_info(
    url: &str,
    database: i64,
    username: Option<String>,
    password: Option<String>,
) -> LFResult<ConnectionInfo> {
    let mut connection_info = url
        .into_connection_info()
        .map_err(|error| redis_error("address parse", error))?;
    connection_info.redis = redis_connection_info(database, username, password);
    Ok(connection_info)
}

fn redis_error(operation: &str, error: redis::RedisError) -> LiteflowError {
    LiteflowError::Rule(format!("redis {operation} error: {error}"))
}
