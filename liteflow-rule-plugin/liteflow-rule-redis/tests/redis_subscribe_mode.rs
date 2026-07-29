use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use liteflow_core::rule_plugin::{RuleFormat, RuleSourceWatcher};
use liteflow_core::serde_json::Value;
use liteflow_core::{FlowBus, cmp};
use liteflow_rule_redis::parser::redis::mode::polling::RedisParserPollingMode;
use liteflow_rule_redis::parser::redis::mode::subscribe::RedisParserSubscribeMode;
use liteflow_rule_redis::parser::redis::mode::{
    RClient, RedisMode, RedisParserHelper, RedisParserMode,
};
use liteflow_rule_redis::parser::redis::vo::RedisParserVO;
use liteflow_rule_redis::{RedisRuleSource, parser::redis::RedisXmlELParser};
use redis::Commands;

struct RedisServer {
    child: Child,
    url: String,
    _directory: tempfile::TempDir,
}

struct RedisSentinelServer {
    child: Child,
    address: String,
    log_path: std::path::PathBuf,
    _directory: tempfile::TempDir,
}

impl Drop for RedisSentinelServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

struct RedisClusterServers {
    children: Vec<Child>,
    addresses: Vec<String>,
    _directories: Vec<tempfile::TempDir>,
}

impl Drop for RedisClusterServers {
    fn drop(&mut self) {
        for child in &mut self.children {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for RedisServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn start_redis_server() -> Option<RedisServer> {
    let redis_server = which_redis_server()?;
    let listener = TcpListener::bind("127.0.0.1:0").ok()?;
    let port = listener.local_addr().ok()?.port();
    drop(listener);

    let directory = tempfile::tempdir().ok()?;
    let child = Command::new(redis_server)
        .args([
            "--bind",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "--save",
            "",
            "--appendonly",
            "no",
            "--notify-keyspace-events",
            "KEA",
            "--dir",
            directory.path().to_str()?,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    Some(RedisServer {
        child,
        url: format!("redis://127.0.0.1:{port}/0"),
        _directory: directory,
    })
}

fn start_redis_replica(master: &RedisServer) -> Option<RedisServer> {
    let redis_server = which_redis_server()?;
    let listener = TcpListener::bind("127.0.0.1:0").ok()?;
    let port = listener.local_addr().ok()?.port();
    drop(listener);
    let master_port = master
        .url
        .split(':')
        .nth(2)?
        .split('/')
        .next()?
        .parse::<u16>()
        .ok()?;
    let directory = tempfile::tempdir().ok()?;
    let child = Command::new(redis_server)
        .args([
            "--bind",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "--save",
            "",
            "--appendonly",
            "no",
            "--notify-keyspace-events",
            "KEA",
            "--replicaof",
            "127.0.0.1",
            &master_port.to_string(),
            "--dir",
            directory.path().to_str()?,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    Some(RedisServer {
        child,
        url: format!("redis://127.0.0.1:{port}/0"),
        _directory: directory,
    })
}

fn which_redis_server() -> Option<String> {
    let output = Command::new("sh")
        .args(["-c", "command -v redis-server"])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|path| !path.is_empty())
}

fn which_redis_cli() -> Option<String> {
    let output = Command::new("sh")
        .args(["-c", "command -v redis-cli"])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|path| !path.is_empty())
}

fn available_port() -> Option<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").ok()?;
    listener.local_addr().ok().map(|address| address.port())
}

fn available_cluster_port(excluded: &[u16]) -> Option<u16> {
    // Redis Cluster 还会占用 port + 10000 的总线端口，因此两者都要可绑定。
    (20_000_u16..45_000_u16).find(|port| {
        !excluded.contains(port)
            && TcpListener::bind(("127.0.0.1", *port)).is_ok()
            && TcpListener::bind(("127.0.0.1", *port + 10_000)).is_ok()
    })
}

async fn start_redis_sentinel(master: &RedisServer) -> Option<RedisSentinelServer> {
    let redis_server = which_redis_server()?;
    let sentinel_port = available_port()?;
    let master_port = master
        .url
        .split(':')
        .nth(2)?
        .split('/')
        .next()?
        .parse::<u16>()
        .ok()?;
    let directory = tempfile::tempdir().ok()?;
    let config_path = directory.path().join("sentinel.conf");
    let log_path = directory.path().join("sentinel.log");
    std::fs::write(
        &config_path,
        format!(
            "port {sentinel_port}\n\
             bind 127.0.0.1\n\
             protected-mode no\n\
             dir {}\n\
             logfile \"{}\"\n\
             sentinel monitor liteflow-master 127.0.0.1 {master_port} 1\n\
             sentinel down-after-milliseconds liteflow-master 500\n\
             sentinel failover-timeout liteflow-master 2000\n",
            directory.path().display(),
            log_path.display()
        ),
    )
    .ok()?;
    let child = Command::new(redis_server)
        .arg(&config_path)
        .arg("--sentinel")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let server = RedisSentinelServer {
        child,
        address: format!("127.0.0.1:{sentinel_port}"),
        log_path,
        _directory: directory,
    };

    let sentinel_url = format!("redis://{}/0", server.address);
    let _ = connect(&sentinel_url).await;
    Some(server)
}

async fn start_redis_cluster(replica_count: usize) -> Option<RedisClusterServers> {
    let redis_server = which_redis_server()?;
    let redis_cli = which_redis_cli()?;
    let mut ports = Vec::new();
    let node_count = 3 * (replica_count + 1);
    for _ in 0..node_count {
        ports.push(available_cluster_port(&ports)?);
    }

    let mut children = Vec::new();
    let mut directories = Vec::new();
    for port in &ports {
        let directory = tempfile::tempdir().ok()?;
        let child = Command::new(&redis_server)
            .args([
                "--bind",
                "127.0.0.1",
                "--protected-mode",
                "no",
                "--port",
                &port.to_string(),
                "--cluster-enabled",
                "yes",
                "--cluster-config-file",
                "nodes.conf",
                "--cluster-node-timeout",
                "1000",
                "--save",
                "",
                "--appendonly",
                "no",
                "--notify-keyspace-events",
                "KEA",
                "--dir",
                directory.path().to_str()?,
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        children.push(child);
        directories.push(directory);
    }

    for port in &ports {
        let _ = connect(&format!("redis://127.0.0.1:{port}/0")).await;
    }
    let addresses = ports
        .iter()
        .map(|port| format!("127.0.0.1:{port}"))
        .collect::<Vec<_>>();
    let status = Command::new(redis_cli)
        .args(["--cluster", "create"])
        .args(&addresses)
        .args([
            "--cluster-replicas",
            &replica_count.to_string(),
            "--cluster-yes",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?;
    if !status.success() {
        return None;
    }
    Some(RedisClusterServers {
        children,
        addresses,
        _directories: directories,
    })
}

fn cluster_master_for_slot(nodes: &str, slot: u16) -> Option<String> {
    for line in nodes.lines() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 9 || !fields[2].split(',').any(|flag| flag == "master") {
            continue;
        }
        let owns_slot = fields[8..].iter().any(|range| {
            if range.starts_with('[') {
                return false;
            }
            let mut bounds = range.split('-');
            let start = bounds.next().and_then(|value| value.parse::<u16>().ok());
            let end = bounds
                .next()
                .and_then(|value| value.parse::<u16>().ok())
                .or(start);
            start
                .zip(end)
                .is_some_and(|(start, end)| (start..=end).contains(&slot))
        });
        if owns_slot {
            return fields[1].split('@').next().map(str::to_string);
        }
    }
    None
}

fn redis_cluster_slot(key: &[u8]) -> u16 {
    let hash_key = key
        .iter()
        .position(|byte| *byte == b'{')
        .and_then(|open| {
            key[open + 1..]
                .iter()
                .position(|byte| *byte == b'}')
                .filter(|close| *close > 0)
                .map(|close| &key[open + 1..open + 1 + close])
        })
        .unwrap_or(key);
    crc16::State::<crc16::XMODEM>::calculate(hash_key) % 16_384
}

fn cluster_master_nodes(nodes: &str) -> Vec<(String, String)> {
    nodes
        .lines()
        .filter_map(|line| {
            let fields = line.split_whitespace().collect::<Vec<_>>();
            if fields.len() < 3
                || !fields[2].split(',').any(|flag| flag == "master")
                || fields[2]
                    .split(',')
                    .any(|flag| matches!(flag, "fail" | "fail?" | "handshake" | "noaddr"))
            {
                return None;
            }
            Some((
                fields[0].to_string(),
                fields[1].split('@').next()?.to_string(),
            ))
        })
        .collect()
}

async fn connect(url: &str) -> redis::Connection {
    let client = redis::Client::open(url).expect("Redis URL 应有效");
    for _ in 0..100 {
        if let Ok(connection) = client.get_connection() {
            return connection;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("本地 Redis 未在超时前就绪");
}

fn redis_integration_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

fn create_acl_user(connection: &mut redis::Connection, username: &str, password: &str) {
    let _: () = redis::cmd("ACL")
        .arg("SETUSER")
        .arg(username)
        .arg("on")
        .arg(format!(">{password}"))
        .arg("~*")
        .arg("&*")
        .arg("+@all")
        .query(connection)
        .expect("Redis ACL 用户应创建成功");
}

#[tokio::test]
async fn single_mode_uses_acl_credentials_for_fetch_and_subscription() {
    let _guard = redis_integration_lock().lock().await;
    let Some(server) = start_redis_server() else {
        eprintln!("redis-server 不可用，跳过真实单点 ACL 测试");
        return;
    };
    let chain_key = "liteflow:test:single:acl";
    let mut administration_connection = connect(&server.url).await;
    let _: () = redis::cmd("HSET")
        .arg(chain_key)
        .arg("single_acl_before:true")
        .arg("THEN(a)")
        .query(&mut administration_connection)
        .expect("写入单点 ACL 初始规则应成功");
    create_acl_user(
        &mut administration_connection,
        "liteflow_user",
        "liteflow_secret",
    );

    let port = server
        .url
        .split(':')
        .nth(2)
        .and_then(|part| part.split('/').next())
        .and_then(|port| port.parse().ok());
    let config = RedisParserVO {
        host: Some("127.0.0.1".to_string()),
        port,
        username: Some("liteflow_user".to_string()),
        password: Some("liteflow_secret".to_string()),
        chain_data_base: Some(0),
        chain_key: Some(chain_key.to_string()),
        ..RedisParserVO::default()
    };
    let parser = RedisXmlELParser::new(config.clone()).expect("单点 ACL 解析器应创建");
    let xml = parser
        .get_content()
        .expect("单点客户端应使用用户名和密码读取 Hash");
    assert!(xml.contains("<chain id=\"single_acl_before\" enable=\"true\">"));

    let mut wrong_config = config.clone();
    wrong_config.password = Some("wrong_secret".to_string());
    let error = RedisXmlELParser::new(wrong_config)
        .expect("错误凭证不应在纯配置阶段伪失败")
        .get_content()
        .expect_err("错误 ACL 密码必须被 Redis 拒绝");
    assert!(
        error.to_string().contains("WRONGPASS")
            || error.to_string().contains("Authentication failed")
            || error.to_string().contains("authentication failed")
            || error.to_string().contains("AuthenticationFailed")
            || error.to_string().contains("invalid username-password"),
        "错误 ACL 密码应产生认证错误，实际为: {error}"
    );

    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    let watcher = RuleSourceWatcher::new(bus.clone(), Arc::new(parser))
        .await
        .expect("单点 ACL 初始规则应装载");
    let mode = RedisParserSubscribeMode::from_config(config).expect("单点 ACL 订阅模式应创建");
    let handle = mode
        .listen_redis(watcher)
        .await
        .expect("单点 ACL keyspace 订阅应建立");
    let _: () = redis::pipe()
        .atomic()
        .cmd("HDEL")
        .arg(chain_key)
        .arg("single_acl_before:true")
        .ignore()
        .cmd("HSET")
        .arg(chain_key)
        .arg("single_acl_after:true")
        .arg("THEN(a)")
        .ignore()
        .query(&mut administration_connection)
        .expect("更新单点 ACL Hash 应成功");
    for _ in 0..100 {
        if bus.contains_chain("single_acl_after") && !bus.contains_chain("single_acl_before") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(bus.contains_chain("single_acl_after"));
    assert!(!bus.contains_chain("single_acl_before"));
    handle.stop().await;
}

#[tokio::test]
async fn single_mode_preserves_java_password_only_and_username_only_rules() {
    let _guard = redis_integration_lock().lock().await;
    let Some(password_server) = start_redis_server() else {
        eprintln!("redis-server 不可用，跳过真实 password-only 测试");
        return;
    };
    let chain_key = "liteflow:test:single:password_only";
    let mut administration_connection = connect(&password_server.url).await;
    let _: () = redis::cmd("HSET")
        .arg(chain_key)
        .arg("password_only:true")
        .arg("THEN(a)")
        .query(&mut administration_connection)
        .expect("写入 password-only 初始规则应成功");
    let _: () = redis::cmd("ACL")
        .arg("SETUSER")
        .arg("default")
        .arg("on")
        .arg("resetpass")
        .arg(">default_secret")
        .arg("~*")
        .arg("&*")
        .arg("+@all")
        .query(&mut administration_connection)
        .expect("默认用户密码应设置成功");
    let password_port = password_server
        .url
        .split(':')
        .nth(2)
        .and_then(|part| part.split('/').next())
        .and_then(|port| port.parse().ok());
    let password_config = RedisParserVO {
        host: Some("127.0.0.1".to_string()),
        port: password_port,
        password: Some("default_secret".to_string()),
        chain_data_base: Some(0),
        chain_key: Some(chain_key.to_string()),
        ..RedisParserVO::default()
    };
    assert!(
        RedisXmlELParser::new(password_config)
            .expect("password-only 解析器应创建")
            .get_content()
            .expect("只有密码时应认证默认用户")
            .contains("<chain id=\"password_only\" enable=\"true\">")
    );

    let Some(username_only_server) = start_redis_server() else {
        eprintln!("redis-server 不可用，跳过真实 username-only 回退测试");
        return;
    };
    let username_only_key = "liteflow:test:single:username_only";
    let mut username_only_connection = connect(&username_only_server.url).await;
    let _: () = redis::cmd("HSET")
        .arg(username_only_key)
        .arg("username_only:true")
        .arg("THEN(a)")
        .query(&mut username_only_connection)
        .expect("写入 username-only 初始规则应成功");
    let username_only_port = username_only_server
        .url
        .split(':')
        .nth(2)
        .and_then(|part| part.split('/').next())
        .and_then(|port| port.parse().ok());
    let username_only_config = RedisParserVO {
        host: Some("127.0.0.1".to_string()),
        port: username_only_port,
        username: Some("java_ignores_without_password".to_string()),
        password: None,
        chain_data_base: Some(0),
        chain_key: Some(username_only_key.to_string()),
        ..RedisParserVO::default()
    };
    assert!(
        RedisXmlELParser::new(username_only_config)
            .expect("username-only 解析器应创建")
            .get_content()
            .expect("Java 语义要求无密码时忽略用户名")
            .contains("<chain id=\"username_only\" enable=\"true\">")
    );
}

#[tokio::test]
async fn sentinel_mode_reads_hash_from_discovered_master() {
    let _guard = redis_integration_lock().lock().await;
    let Some(master) = start_redis_server() else {
        eprintln!("redis-server 不可用，跳过真实 Sentinel 测试");
        return;
    };
    let Some(sentinel) = start_redis_sentinel(&master).await else {
        eprintln!("Redis Sentinel 启动失败，跳过真实 Sentinel 测试");
        return;
    };
    let chain_key = "liteflow:test:sentinel:chains";
    let mut master_connection = connect(&master.url).await;
    let _: () = redis::cmd("HSET")
        .arg(chain_key)
        .arg("sentinel_chain:true")
        .arg("THEN(a)")
        .query(&mut master_connection)
        .expect("向 Sentinel 主节点写入 Hash 应成功");
    create_acl_user(&mut master_connection, "liteflow_user", "liteflow_secret");

    let config = RedisParserVO {
        redis_mode: RedisMode::Sentinel,
        master_name: Some("liteflow-master".to_string()),
        sentinel_address: vec![sentinel.address.clone()],
        username: Some("liteflow_user".to_string()),
        password: Some("liteflow_secret".to_string()),
        chain_data_base: Some(0),
        chain_key: Some(chain_key.to_string()),
        ..RedisParserVO::default()
    };
    let parser = RedisXmlELParser::new(config.clone()).expect("Sentinel 解析器应创建成功");
    let xml = parser
        .get_content()
        .expect("应通过 Sentinel 发现主节点并使用 ACL 读取 Hash");
    assert!(xml.contains("<chain id=\"sentinel_chain\" enable=\"true\">"));
    let mut wrong_config = config.clone();
    wrong_config.password = Some("wrong_secret".to_string());
    assert!(
        RedisXmlELParser::new(wrong_config)
            .expect("错误 Sentinel 凭证配置仍可构造")
            .get_content()
            .is_err(),
        "错误 Sentinel 节点密码必须被 Redis 拒绝"
    );

    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    let watcher = RuleSourceWatcher::new(bus.clone(), Arc::new(parser))
        .await
        .expect("Sentinel 初始规则应装载");
    let subscribe_mode =
        RedisParserSubscribeMode::from_config(config).expect("Sentinel 订阅模式应创建");
    let handle = subscribe_mode
        .listen_redis(watcher)
        .await
        .expect("Sentinel 主节点 keyspace 订阅应建立");
    let _: () = redis::pipe()
        .atomic()
        .cmd("HDEL")
        .arg(chain_key)
        .arg("sentinel_chain:true")
        .ignore()
        .cmd("HSET")
        .arg(chain_key)
        .arg("sentinel_after:true")
        .arg("THEN(a)")
        .ignore()
        .query(&mut master_connection)
        .expect("更新 Sentinel 主节点 Hash 应成功");
    for _ in 0..100 {
        if bus.contains_chain("sentinel_after") && !bus.contains_chain("sentinel_chain") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(bus.contains_chain("sentinel_after"));
    assert!(!bus.contains_chain("sentinel_chain"));
    handle.stop().await;
}

#[tokio::test]
async fn sentinel_subscription_follows_promoted_master() {
    let _guard = redis_integration_lock().lock().await;
    let Some(mut master) = start_redis_server() else {
        eprintln!("redis-server 不可用，跳过真实 Sentinel 故障转移测试");
        return;
    };
    let Some(replica) = start_redis_replica(&master) else {
        eprintln!("Redis replica 启动失败，跳过真实 Sentinel 故障转移测试");
        return;
    };
    let _ = connect(&replica.url).await;
    let Some(sentinel) = start_redis_sentinel(&master).await else {
        eprintln!("Redis Sentinel 启动失败，跳过真实 Sentinel 故障转移测试");
        return;
    };
    let chain_key = "liteflow:test:sentinel:failover";
    let mut master_connection = connect(&master.url).await;
    let _: () = redis::cmd("HSET")
        .arg(chain_key)
        .arg("failover_before:true")
        .arg("THEN(a)")
        .query(&mut master_connection)
        .expect("向故障转移前主节点写入 Hash 应成功");

    // 等待副本确认收到初始规则，避免在复制尚未建立时主动终止主节点。
    let mut replica_connection = connect(&replica.url).await;
    for _ in 0..300 {
        let value: Option<String> = redis::cmd("HGET")
            .arg(chain_key)
            .arg("failover_before:true")
            .query(&mut replica_connection)
            .unwrap_or(None);
        if value.as_deref() == Some("THEN(a)") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let mut sentinel_connection = connect(&format!("redis://{}/0", sentinel.address)).await;
    let mut sentinel_knows_replica = false;
    for _ in 0..500 {
        let replicas: redis::RedisResult<Vec<std::collections::HashMap<String, String>>> =
            redis::cmd("SENTINEL")
                .arg("REPLICAS")
                .arg("liteflow-master")
                .query(&mut sentinel_connection);
        if replicas.is_ok_and(|replicas| {
            replicas.iter().any(|replica| {
                replica
                    .get("flags")
                    .is_some_and(|flags| flags.split(',').all(|flag| flag == "slave"))
                    && replica
                        .get("master-link-status")
                        .is_some_and(|status| status == "ok")
            })
        }) {
            sentinel_knows_replica = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(
        sentinel_knows_replica,
        "Sentinel 必须先发现副本再触发故障转移"
    );

    let config = RedisParserVO {
        redis_mode: RedisMode::Sentinel,
        master_name: Some("liteflow-master".to_string()),
        sentinel_address: vec![sentinel.address.clone()],
        chain_data_base: Some(0),
        chain_key: Some(chain_key.to_string()),
        ..RedisParserVO::default()
    };
    let parser = RedisXmlELParser::new(config.clone()).expect("Sentinel 解析器应创建");
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    let watcher = RuleSourceWatcher::new(bus.clone(), Arc::new(parser))
        .await
        .expect("故障转移前规则应装载");
    let mode = RedisParserSubscribeMode::from_config(config).expect("Sentinel 订阅模式应创建");
    let handle = mode
        .listen_redis(watcher)
        .await
        .expect("故障转移前订阅应建立");

    master.child.kill().expect("应终止旧主节点");
    let _ = master.child.wait();

    // Sentinel 将副本提升为主节点后，写命令才会成功。
    let mut promoted = false;
    for _ in 0..2000 {
        let client = redis::Client::open(replica.url.as_str()).expect("副本 URL 应有效");
        if let Ok(mut current_connection) = client.get_connection() {
            let result: redis::RedisResult<()> = redis::cmd("SET")
                .arg("liteflow:test:sentinel:promotion_probe")
                .arg("ready")
                .query(&mut current_connection);
            if result.is_ok() {
                replica_connection = current_connection;
                promoted = true;
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    if !promoted {
        let master_state: redis::RedisResult<redis::Value> = redis::cmd("SENTINEL")
            .arg("MASTER")
            .arg("liteflow-master")
            .query(&mut sentinel_connection);
        let replica_state: redis::RedisResult<redis::Value> = redis::cmd("SENTINEL")
            .arg("REPLICAS")
            .arg("liteflow-master")
            .query(&mut sentinel_connection);
        eprintln!("Sentinel master state: {master_state:?}");
        eprintln!("Sentinel replica state: {replica_state:?}");
        eprintln!(
            "Sentinel log:\n{}",
            std::fs::read_to_string(&sentinel.log_path).unwrap_or_default()
        );
    }
    assert!(promoted, "Sentinel 应在超时前提升副本");

    // 给拓扑核对任务一个周期，确保新主节点订阅先建立再写入正式规则。
    tokio::time::sleep(Duration::from_millis(1200)).await;
    let _: () = redis::pipe()
        .atomic()
        .cmd("HDEL")
        .arg(chain_key)
        .arg("failover_before:true")
        .ignore()
        .cmd("HSET")
        .arg(chain_key)
        .arg("failover_after:true")
        .arg("THEN(a)")
        .ignore()
        .query(&mut replica_connection)
        .expect("应向提升后的新主节点写入规则");
    for _ in 0..300 {
        if bus.contains_chain("failover_after") && !bus.contains_chain("failover_before") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(bus.contains_chain("failover_after"));
    assert!(!bus.contains_chain("failover_before"));
    handle.stop().await;
}

#[tokio::test]
async fn cluster_mode_routes_hash_commands_to_owning_node() {
    let _guard = redis_integration_lock().lock().await;
    let Some(cluster) = start_redis_cluster(0).await else {
        eprintln!("redis-server/redis-cli 不可用，跳过真实 Cluster 测试");
        return;
    };
    let chain_key = "liteflow:test:cluster:chains";
    let script_key = "liteflow:test:cluster:scripts";
    // 通过原生 ClusterConnection 发出命令，验证写入和读取都经过槽位路由。
    let cluster_urls = cluster
        .addresses
        .iter()
        .map(|address| format!("redis://{address}"))
        .collect::<Vec<_>>();
    let cluster_client =
        redis::cluster::ClusterClient::new(cluster_urls).expect("Cluster 配置应有效");
    let mut connection = cluster_client
        .get_connection()
        .expect("Cluster 拓扑应完成发现");
    let _: () = redis::cmd("HSET")
        .arg(chain_key)
        .arg("cluster_chain:true")
        .arg("THEN(cluster_script)")
        .query(&mut connection)
        .expect("Cluster Chain Hash 写入应成功");
    let _: () = redis::cmd("HSET")
        .arg(script_key)
        .arg("cluster_script:script:Cluster Script:rhai:true")
        .arg("6 * 7")
        .query(&mut connection)
        .expect("Cluster Script Hash 写入应成功");
    for address in &cluster.addresses {
        let mut node_connection = connect(&format!("redis://{address}/0")).await;
        create_acl_user(&mut node_connection, "liteflow_user", "liteflow_secret");
    }
    let client = RClient::cluster(
        &cluster.addresses,
        Some("liteflow_user".to_string()),
        Some("liteflow_secret".to_string()),
    )
    .expect("带 ACL 的三节点 Cluster 客户端应创建成功");
    assert_eq!(
        client
            .hget(chain_key, "cluster_chain:true")
            .expect("RClient 应按槽位读取字段")
            .as_deref(),
        Some("THEN(cluster_script)")
    );

    let config = RedisParserVO {
        redis_mode: RedisMode::Cluster,
        cluster_node_address: cluster.addresses.clone(),
        username: Some("liteflow_user".to_string()),
        password: Some("liteflow_secret".to_string()),
        chain_key: Some(chain_key.to_string()),
        script_key: Some(script_key.to_string()),
        ..RedisParserVO::default()
    };
    let parser = RedisXmlELParser::new(config.clone()).expect("Cluster 解析器应创建成功");
    let xml = parser.get_content().expect("Cluster Hash 应聚合为 XML");
    assert!(xml.contains("<chain id=\"cluster_chain\" enable=\"true\">"));
    assert!(xml.contains("<node id=\"cluster_script\""));
    let bad_client = RClient::cluster(
        &cluster.addresses,
        Some("liteflow_user".to_string()),
        Some("wrong_secret".to_string()),
    )
    .expect("错误 Cluster 凭证仍可通过离线配置校验");
    assert!(
        bad_client.hget(chain_key, "cluster_chain:true").is_err(),
        "错误 Cluster ACL 密码必须被节点拒绝"
    );

    let bus = FlowBus::new();
    let watcher = RuleSourceWatcher::new(bus.clone(), Arc::new(parser))
        .await
        .expect("Cluster 初始规则应装载");
    let subscribe_mode =
        RedisParserSubscribeMode::from_config(config).expect("Cluster 订阅模式应创建");
    let handle = subscribe_mode
        .listen_redis(watcher)
        .await
        .expect("全部 Cluster 主节点 keyspace 订阅应建立");
    let _: () = redis::cmd("HDEL")
        .arg(chain_key)
        .arg("cluster_chain:true")
        .query(&mut connection)
        .expect("删除 Cluster Chain 字段应通过槽位路由");
    let _: () = redis::cmd("HSET")
        .arg(chain_key)
        .arg("cluster_after:true")
        .arg("THEN(cluster_script)")
        .query(&mut connection)
        .expect("更新 Cluster Hash 应通过槽位路由");
    for _ in 0..200 {
        if bus.contains_chain("cluster_after") && !bus.contains_chain("cluster_chain") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(bus.contains_chain("cluster_after"));
    assert!(!bus.contains_chain("cluster_chain"));
    handle.stop().await;
}

#[tokio::test]
async fn cluster_subscription_follows_promoted_replica() {
    let _guard = redis_integration_lock().lock().await;
    let Some(mut cluster) = start_redis_cluster(1).await else {
        eprintln!("redis-server/redis-cli 不可用，跳过真实 Cluster 故障转移测试");
        return;
    };
    let chain_key = "liteflow:test:cluster:failover";
    let cluster_urls = cluster
        .addresses
        .iter()
        .map(|address| format!("redis://{address}"))
        .collect::<Vec<_>>();
    let cluster_client = redis::cluster::ClusterClient::new(cluster_urls.clone())
        .expect("六节点 Cluster 配置应有效");
    let mut connection = cluster_client
        .get_connection()
        .expect("六节点 Cluster 拓扑应完成发现");
    let _: () = redis::cmd("HSET")
        .arg(chain_key)
        .arg("cluster_failover_before:true")
        .arg("THEN(a)")
        .query(&mut connection)
        .expect("故障转移前 Cluster Hash 应写入");

    let nodes: String = redis::cmd("CLUSTER")
        .arg("NODES")
        .query(&mut connection)
        .expect("应读取 Cluster 节点视图");
    let slot = redis_cluster_slot(chain_key.as_bytes());
    let owning_master = cluster_master_for_slot(&nodes, slot).expect("应定位测试 key 所属主节点");
    let owning_index = cluster
        .addresses
        .iter()
        .position(|address| address == &owning_master)
        .expect("所属主节点应来自测试集群");
    let mut replica_ready = false;
    for _ in 0..1000 {
        let client =
            redis::Client::open(format!("redis://{owning_master}/0")).expect("主节点 URL 应有效");
        if let Ok(mut owner_connection) = client.get_connection() {
            let replication: redis::RedisResult<String> = redis::cmd("INFO")
                .arg("replication")
                .query(&mut owner_connection);
            if replication.is_ok_and(|info| {
                info.contains("connected_slaves:1") && info.contains("state=online")
            }) {
                replica_ready = true;
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(replica_ready, "所属主节点的副本必须先完成同步");

    let config = RedisParserVO {
        redis_mode: RedisMode::Cluster,
        cluster_node_address: cluster.addresses.clone(),
        chain_key: Some(chain_key.to_string()),
        ..RedisParserVO::default()
    };
    let parser = RedisXmlELParser::new(config.clone()).expect("Cluster 故障转移解析器应创建");
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    let watcher = RuleSourceWatcher::new(bus.clone(), Arc::new(parser))
        .await
        .expect("Cluster 故障转移前规则应装载");
    let mode =
        RedisParserSubscribeMode::from_config(config.clone()).expect("Cluster 订阅模式应创建");
    let handle = mode
        .listen_redis(watcher)
        .await
        .expect("Cluster 故障转移前全部主节点订阅应建立");

    cluster.children[owning_index]
        .kill()
        .expect("应终止测试 key 所属主节点");
    let _ = cluster.children[owning_index].wait();

    let routed_client =
        RClient::cluster(&cluster.addresses, None, None).expect("故障后的 Cluster 客户端应可构造");
    let mut promoted = false;
    for _ in 0..1500 {
        if routed_client
            .hget(chain_key, "cluster_failover_before:true")
            .is_ok_and(|value| value.as_deref() == Some("THEN(a)"))
        {
            promoted = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    if !promoted {
        for address in &cluster.addresses {
            if address == &owning_master {
                continue;
            }
            let client =
                redis::Client::open(format!("redis://{address}/0")).expect("存活节点 URL 应有效");
            if let Ok(mut live_connection) = client.get_connection() {
                let current_nodes: redis::RedisResult<String> = redis::cmd("CLUSTER")
                    .arg("NODES")
                    .query(&mut live_connection);
                eprintln!("Cluster nodes after failover timeout:\n{current_nodes:?}");
                break;
            }
        }
    }
    assert!(promoted, "Cluster 应在超时前提升所属槽位的副本");

    // 等待订阅器发现新的主节点集合并完成先建后换。
    tokio::time::sleep(Duration::from_millis(1200)).await;
    let mut updated = false;
    for _ in 0..300 {
        let client = match redis::cluster::ClusterClient::new(cluster_urls.clone()) {
            Ok(client) => client,
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(10)).await;
                continue;
            }
        };
        let mut current_connection = match client.get_connection() {
            Ok(connection) => connection,
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(10)).await;
                continue;
            }
        };
        let removed: redis::RedisResult<i64> = redis::cmd("HDEL")
            .arg(chain_key)
            .arg("cluster_failover_before:true")
            .query(&mut current_connection);
        let inserted: redis::RedisResult<i64> = redis::cmd("HSET")
            .arg(chain_key)
            .arg("cluster_failover_after:true")
            .arg("THEN(a)")
            .query(&mut current_connection);
        if removed.is_ok() && inserted.is_ok() {
            updated = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(updated, "提升后的 Cluster 主节点应可更新规则");
    for _ in 0..500 {
        if bus.contains_chain("cluster_failover_after")
            && !bus.contains_chain("cluster_failover_before")
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(bus.contains_chain("cluster_failover_after"));
    assert!(!bus.contains_chain("cluster_failover_before"));
    handle.stop().await;
}

#[tokio::test]
async fn cluster_subscription_survives_online_slot_resharding() {
    let _guard = redis_integration_lock().lock().await;
    let Some(cluster) = start_redis_cluster(0).await else {
        eprintln!("redis-server/redis-cli 不可用，跳过真实 Cluster 重分片测试");
        return;
    };
    let chain_key = "liteflow:test:cluster:resharding";
    let cluster_urls = cluster
        .addresses
        .iter()
        .map(|address| format!("redis://{address}"))
        .collect::<Vec<_>>();
    let cluster_client = redis::cluster::ClusterClient::new(cluster_urls.clone())
        .expect("三节点 Cluster 配置应有效");
    let mut connection = cluster_client
        .get_connection()
        .expect("三节点 Cluster 拓扑应完成发现");
    let _: () = redis::cmd("HSET")
        .arg(chain_key)
        .arg("resharding_before:true")
        .arg("THEN(a)")
        .query(&mut connection)
        .expect("重分片前规则应写入");

    let nodes: String = redis::cmd("CLUSTER")
        .arg("NODES")
        .query(&mut connection)
        .expect("应读取重分片前节点视图");
    let slot = redis_cluster_slot(chain_key.as_bytes());
    let source_address = cluster_master_for_slot(&nodes, slot).expect("应定位重分片源主节点");
    let masters = cluster_master_nodes(&nodes);
    let source_id = masters
        .iter()
        .find(|(_, address)| address == &source_address)
        .map(|(id, _)| id.clone())
        .expect("应定位源主节点 id");
    let (target_id, target_address) = masters
        .iter()
        .find(|(_, address)| address != &source_address)
        .cloned()
        .expect("应选择另一个主节点作为目标");

    let config = RedisParserVO {
        redis_mode: RedisMode::Cluster,
        cluster_node_address: cluster.addresses.clone(),
        chain_key: Some(chain_key.to_string()),
        ..RedisParserVO::default()
    };
    let parser = RedisXmlELParser::new(config.clone()).expect("重分片解析器应创建");
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    let watcher = RuleSourceWatcher::new(bus.clone(), Arc::new(parser))
        .await
        .expect("重分片前规则应装载");
    let mode = RedisParserSubscribeMode::from_config(config).expect("重分片订阅模式应创建");
    let handle = mode
        .listen_redis(watcher)
        .await
        .expect("重分片前全部主节点订阅应建立");

    let mut source_connection = connect(&format!("redis://{source_address}/0")).await;
    let mut target_connection = connect(&format!("redis://{target_address}/0")).await;
    let _: () = redis::cmd("CLUSTER")
        .arg("SETSLOT")
        .arg(slot)
        .arg("IMPORTING")
        .arg(&source_id)
        .query(&mut target_connection)
        .expect("目标节点应进入 IMPORTING 状态");
    let _: () = redis::cmd("CLUSTER")
        .arg("SETSLOT")
        .arg(slot)
        .arg("MIGRATING")
        .arg(&target_id)
        .query(&mut source_connection)
        .expect("源节点应进入 MIGRATING 状态");
    let mut target_parts = target_address.rsplitn(2, ':');
    let target_port = target_parts
        .next()
        .and_then(|value| value.parse::<u16>().ok())
        .expect("目标端口应有效");
    let target_host = target_parts.next().expect("目标主机应有效");
    let migration: String = redis::cmd("MIGRATE")
        .arg(target_host)
        .arg(target_port)
        .arg("")
        .arg(0)
        .arg(5000)
        .arg("KEYS")
        .arg(chain_key)
        .query(&mut source_connection)
        .expect("Hash key 应在线迁移到目标主节点");
    assert_eq!(migration, "OK");
    for (_, address) in &masters {
        let mut node_connection = connect(&format!("redis://{address}/0")).await;
        let _: () = redis::cmd("CLUSTER")
            .arg("SETSLOT")
            .arg(slot)
            .arg("NODE")
            .arg(&target_id)
            .query(&mut node_connection)
            .expect("所有主节点应提交新槽位归属");
    }

    let routed_client =
        RClient::cluster(&cluster.addresses, None, None).expect("重分片后客户端应可构造");
    assert_eq!(
        routed_client
            .hget(chain_key, "resharding_before:true")
            .expect("重分片后应从新主节点读取 Hash")
            .as_deref(),
        Some("THEN(a)")
    );
    let post_reshard_client =
        redis::cluster::ClusterClient::new(cluster_urls).expect("重分片后 Cluster 配置应有效");
    let mut post_reshard_connection = post_reshard_client
        .get_connection()
        .expect("重分片后应刷新槽位视图");
    let _: () = redis::cmd("HDEL")
        .arg(chain_key)
        .arg("resharding_before:true")
        .query(&mut post_reshard_connection)
        .expect("应从新主节点删除旧规则");
    let _: () = redis::cmd("HSET")
        .arg(chain_key)
        .arg("resharding_after:true")
        .arg("THEN(a)")
        .query(&mut post_reshard_connection)
        .expect("应向新主节点写入新规则");
    for _ in 0..500 {
        if bus.contains_chain("resharding_after") && !bus.contains_chain("resharding_before") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(bus.contains_chain("resharding_after"));
    assert!(!bus.contains_chain("resharding_before"));
    handle.stop().await;
}

#[tokio::test]
async fn keyspace_subscription_reloads_and_removes_managed_chains() {
    let _guard = redis_integration_lock().lock().await;
    let Some(server) = start_redis_server() else {
        eprintln!("redis-server 不可用，跳过真实 Redis 订阅测试");
        return;
    };
    let key = "liteflow:test:flow";
    let mut connection = connect(&server.url).await;
    let before = r#"{"flow":{"chain":[{"id":"before","body":"THEN(a)"}]}}"#.to_string();
    let after = r#"{"flow":{"chain":[{"id":"after","body":"THEN(a)"}]}}"#.to_string();
    let _: () = connection.set(key, before).expect("写入初始规则应成功");

    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    let source = Arc::new(RedisRuleSource {
        url: server.url.clone(),
        key: key.to_string(),
        format: RuleFormat::Json,
    });
    let watcher = RuleSourceWatcher::new(bus.clone(), source)
        .await
        .expect("初始规则装载应成功");
    assert!(bus.contains_chain("before"));

    let handle = RedisParserSubscribeMode::new(&server.url, key)
        .listen_redis(watcher)
        .await
        .expect("Redis keyspace 订阅应成功");
    let _: () = connection.set(key, after).expect("更新规则应成功");

    for _ in 0..100 {
        if bus.contains_chain("after") && !bus.contains_chain("before") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    assert!(bus.contains_chain("after"));
    assert!(!bus.contains_chain("before"));
    handle.stop().await;
}

#[test]
fn blank_subscribe_key_is_rejected_before_connecting() {
    // 订阅参数校验发生在任何网络连接之前，因此不需要启动外部 Redis。
    let mode = RedisParserSubscribeMode::new("redis://127.0.0.1:1/0", " ");
    let error = mode.validate().expect_err("空白 key 必须被拒绝");
    assert!(error.to_string().contains("must not be blank"));
}

#[test]
fn redis_parser_vo_preserves_java_defaults_and_address_aliases() {
    let config: RedisParserVO = serde_json::from_str(
        r#"{"redisMode":"unknown","mode":"unknown","sentinelAddress":"a:1, b:2","clusterAddress":"c:3,d:4"}"#,
    )
    .expect("Redis 配置应完成 serde 解析");
    assert_eq!(config.redis_mode, RedisMode::Single);
    assert_eq!(config.mode, RedisParserMode::Poll);
    assert_eq!(config.polling_interval, 60);
    assert_eq!(config.polling_start_time, 60);
    assert_eq!(config.connection_minimum_idle_size, 2);
    assert_eq!(config.connection_pool_size, 4);
    assert_eq!(config.sentinel_address, ["a:1", "b:2"]);
    assert_eq!(config.cluster_node_address, ["c:3", "d:4"]);
}

#[tokio::test]
async fn redis_hashes_are_aggregated_into_real_xml_rule() {
    let _guard = redis_integration_lock().lock().await;
    let Some(server) = start_redis_server() else {
        eprintln!("redis-server 不可用，跳过真实 Redis Hash 测试");
        return;
    };
    let mut connection = connect(&server.url).await;
    let _: () = redis::cmd("HSET")
        .arg("liteflow:test:chains")
        .arg("hash_chain:true")
        .arg("THEN(hash_script)")
        .query(&mut connection)
        .expect("写入 Chain Hash 应成功");
    let _: () = redis::cmd("HSET")
        .arg("liteflow:test:scripts")
        .arg("hash_script:script:Hash Script:rhai:true")
        .arg("40 + 2")
        .query(&mut connection)
        .expect("写入 Script Hash 应成功");

    let config = RedisParserVO {
        host: Some("127.0.0.1".to_string()),
        port: server
            .url
            .split(':')
            .nth(2)
            .and_then(|part| part.split('/').next())
            .and_then(|port| port.parse().ok()),
        chain_data_base: Some(0),
        chain_key: Some("liteflow:test:chains".to_string()),
        script_data_base: Some(0),
        script_key: Some("liteflow:test:scripts".to_string()),
        ..RedisParserVO::default()
    };
    let parser = RedisXmlELParser::new(config).expect("Hash 解析器应创建成功");
    let xml = parser.get_content().expect("Redis Hash 应聚合为 XML");
    assert!(xml.contains("<chain id=\"hash_chain\" enable=\"true\">"));
    assert!(xml.contains("<node id=\"hash_script\""));

    let bus = FlowBus::new();
    RuleSourceWatcher::new(bus.clone(), Arc::new(parser))
        .await
        .expect("聚合 XML 应由真实 XML parser 装载");
    assert!(bus.contains_chain("hash_chain"));
    assert!(bus.contains_node("hash_script"));
}

#[tokio::test]
async fn polling_mode_reconciles_chain_and_script_hash_changes() {
    let _guard = redis_integration_lock().lock().await;
    let Some(server) = start_redis_server() else {
        eprintln!("redis-server 不可用，跳过真实 Redis polling 测试");
        return;
    };
    let mut connection = connect(&server.url).await;
    let chain_key = "liteflow:test:poll:chains";
    let script_key = "liteflow:test:poll:scripts";
    let _: () = redis::pipe()
        .atomic()
        .cmd("HSET")
        .arg(chain_key)
        .arg("poll_before:true")
        .arg("THEN(poll_script_before)")
        .ignore()
        .cmd("HSET")
        .arg(script_key)
        .arg("poll_script_before:script:Before:rhai:true")
        .arg("1")
        .ignore()
        .query(&mut connection)
        .expect("写入初始 polling Hash 应成功");

    let config = RedisParserVO {
        host: Some("127.0.0.1".to_string()),
        port: server
            .url
            .split(':')
            .nth(2)
            .and_then(|part| part.split('/').next())
            .and_then(|port| port.parse().ok()),
        chain_data_base: Some(0),
        chain_key: Some(chain_key.to_string()),
        script_data_base: Some(0),
        script_key: Some(script_key.to_string()),
        polling_start_time: 0,
        polling_interval: 1,
        ..RedisParserVO::default()
    };
    let mode = RedisParserPollingMode::new(config).expect("Polling 模式应创建成功");
    let bus = FlowBus::new();
    let watcher = RuleSourceWatcher::new(bus.clone(), Arc::new(mode.clone()))
        .await
        .expect("初始 polling 规则应装载成功");
    assert!(bus.contains_chain("poll_before"));
    assert!(bus.contains_node("poll_script_before"));
    let handle = mode.listen_redis(watcher);

    let _: () = redis::pipe()
        .atomic()
        .cmd("HDEL")
        .arg(chain_key)
        .arg("poll_before:true")
        .ignore()
        .cmd("HSET")
        .arg(chain_key)
        .arg("poll_after:true")
        .arg("THEN(poll_script_after)")
        .ignore()
        .cmd("HDEL")
        .arg(script_key)
        .arg("poll_script_before:script:Before:rhai:true")
        .ignore()
        .cmd("HSET")
        .arg(script_key)
        .arg("poll_script_after:script:After:rhai:true")
        .arg("2")
        .ignore()
        .query(&mut connection)
        .expect("更新 polling Hash 应成功");

    for _ in 0..300 {
        if bus.contains_chain("poll_after")
            && !bus.contains_chain("poll_before")
            && bus.contains_node("poll_script_after")
            && !bus.contains_node("poll_script_before")
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    assert!(bus.contains_chain("poll_after"));
    assert!(!bus.contains_chain("poll_before"));
    assert!(bus.contains_node("poll_script_after"));
    assert!(!bus.contains_node("poll_script_before"));
    handle.abort();
    let _ = handle.await;
}
