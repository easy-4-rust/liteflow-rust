//! Redis/MySQL Agent Session 工厂的真实外部后端测试。

use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use agentscope_core::session::{Session, SessionKey};
use agentscope_extensions_session_mysql::MysqlSession;
use agentscope_extensions_session_redis::RedisSession;
use liteflow_agent_core::{
    AgentConfig, AgentSessionFactoryRegistry, MemoryStorageMode, MysqlAgentSessionFactory,
    RedisAgentSessionFactory,
};
use serde_json::{Value, json};

const REDIS_PASSWORD: &str = "liteflow-agent-redis-pass";
const MYSQL_PASSWORD: &str = "liteflow-agent-mysql-pass";

/// 保证测试结束时关闭临时 Redis，避免遗留后台进程。
struct RedisProcess {
    child: Child,
}

/// 保证测试结束时关闭临时 MySQL，避免遗留后台进程。
struct MysqlProcess {
    child: Child,
}

impl Drop for MysqlProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for RedisProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn redis_server_available() -> bool {
    Command::new("redis-server")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn allocate_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .expect("应分配临时 Redis 端口")
        .local_addr()
        .expect("应读取临时 Redis 端口")
        .port()
}

fn command_available(command: &str, version_arg: &str) -> bool {
    Command::new(command)
        .arg(version_arg)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn configure_mysql_layout(command: &mut Command) {
    let Some(path) = std::env::var_os("PATH") else {
        return;
    };
    let Some(mysqld_path) = std::env::split_paths(&path)
        .map(|directory| directory.join("mysqld"))
        .find(|candidate| candidate.is_file())
    else {
        return;
    };
    let Some(prefix) = mysqld_path.parent().and_then(std::path::Path::parent) else {
        return;
    };
    let plugin_dir = prefix.join("lib/plugin");
    if plugin_dir.is_dir() {
        command.arg(format!("--plugin-dir={}", plugin_dir.display()));
    }
    let messages_dir = prefix.join("share/mysql");
    if messages_dir.is_dir() {
        command.arg(format!("--lc-messages-dir={}", messages_dir.display()));
    }
}

fn start_redis(port: u16, work_dir: &std::path::Path, password: &str) -> RedisProcess {
    let child = Command::new("redis-server")
        .args([
            "--bind",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "--save",
            "",
            "--appendonly",
            "no",
            "--requirepass",
            password,
            "--dir",
            work_dir.to_str().expect("临时 Redis 工作目录应为 UTF-8"),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("应启动临时 redis-server");
    RedisProcess { child }
}

async fn connect_redis(redis_url: &str, key_prefix: &str) -> RedisSession {
    let mut last_error = None;
    for _ in 0..40 {
        match RedisSession::with_config(redis_url, key_prefix, 0).await {
            Ok(session) => return session,
            Err(error) => last_error = Some(error),
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!(
        "临时 Redis 未能就绪: {}",
        last_error
            .map(|error| error.to_string())
            .unwrap_or_else(|| "unknown error".to_string())
    );
}

fn initialize_mysql(data_dir: &std::path::Path) {
    let mut command = Command::new("mysqld");
    command.args([
        "--no-defaults",
        "--initialize-insecure",
        &format!("--datadir={}", data_dir.display()),
    ]);
    let status = command
        .arg("--log-error-verbosity=1")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("应初始化临时 MySQL 数据目录");
    assert!(status.success(), "临时 MySQL 初始化必须成功");
}

fn start_mysql(port: u16, data_dir: &std::path::Path) -> MysqlProcess {
    let socket = data_dir.join("mysql.sock");
    let pid_file = data_dir.join("mysql.pid");
    let mut command = Command::new("mysqld");
    command
        .args([
            "--no-defaults",
            "--skip-networking=0",
            "--bind-address=127.0.0.1",
            "--mysqlx=0",
            "--port",
            &port.to_string(),
            "--datadir",
        ])
        .arg(data_dir)
        .arg("--socket")
        .arg(socket)
        .arg("--pid-file")
        .arg(pid_file);
    configure_mysql_layout(&mut command);
    let child = command
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("应启动临时 mysqld");
    MysqlProcess { child }
}

async fn prepare_mysql_database(port: u16, database_name: &str, username: &str, password: &str) {
    let mut last_status = None;
    for _ in 0..80 {
        let status = Command::new("mysqladmin")
            .args([
                "--protocol=TCP",
                "--host=127.0.0.1",
                "--user=root",
                "--port",
                &port.to_string(),
                "ping",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if status.as_ref().is_ok_and(|status| status.success()) {
            let create_status = Command::new("mysql")
                .args([
                    "--protocol=TCP",
                    "--host=127.0.0.1",
                    "--user=root",
                    "--port",
                    &port.to_string(),
                    "--execute",
                    &format!(
                        "CREATE DATABASE `{database_name}`; \
                         CREATE USER '{username}'@'127.0.0.1' IDENTIFIED BY '{password}'; \
                         GRANT ALL PRIVILEGES ON `{database_name}`.* TO '{username}'@'127.0.0.1'; \
                         FLUSH PRIVILEGES"
                    ),
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .expect("应创建 Agent Session 测试数据库");
            assert!(
                create_status.success(),
                "Agent Session 测试数据库必须创建成功"
            );
            return;
        }
        last_status = status.ok().and_then(|status| status.code());
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("临时 MySQL 未能就绪，最后状态码: {last_status:?}");
}

fn get_json_value(
    session: Arc<dyn Session>,
    session_key: SessionKey,
    key: &'static str,
) -> Option<Value> {
    // AgentScope Session 是同步对象安全接口，Redis 实现内部借当前 Tokio Handle
    // 执行异步命令；移到普通线程可避免在运行时工作线程中嵌套 block_on。
    let runtime = tokio::runtime::Handle::current();
    std::thread::spawn(move || {
        let _runtime_guard = runtime.enter();
        session.get_json_value(&session_key, key)
    })
    .join()
    .expect("远端 Session 读取线程不应 panic")
}

fn get_list_json_values(
    session: Arc<dyn Session>,
    session_key: SessionKey,
    key: &'static str,
) -> Option<Vec<Value>> {
    let runtime = tokio::runtime::Handle::current();
    std::thread::spawn(move || {
        let _runtime_guard = runtime.enter();
        session.get_list_json_values(&session_key, key)
    })
    .join()
    .expect("远端 Session 列表读取线程不应 panic")
}

fn session_exists(session: Arc<dyn Session>, session_key: SessionKey) -> bool {
    let runtime = tokio::runtime::Handle::current();
    std::thread::spawn(move || {
        let _runtime_guard = runtime.enter();
        session.exists(&session_key)
    })
    .join()
    .expect("远端 Session exists 线程不应 panic")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redis_factory_drives_real_agentscope_session_persistence() {
    if !redis_server_available() {
        eprintln!("redis-server 不可用，跳过 Agent Session 真实 Redis 测试");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("应创建 Redis 临时目录");
    let port = allocate_port();
    let _redis_process = start_redis(port, temp_dir.path(), REDIS_PASSWORD);
    let redis_url = format!("redis://:{REDIS_PASSWORD}@127.0.0.1:{port}/");
    let wrong_redis_url = format!("redis://:wrong-password@127.0.0.1:{port}/");
    let bean_name = format!("agentRedisSession{port}");
    let key_prefix = format!("liteflow:agent:test:{port}:");

    let redis_session = connect_redis(&redis_url, &key_prefix).await;
    assert!(
        RedisSession::with_config(&wrong_redis_url, &key_prefix, 0)
            .await
            .is_err(),
        "错误 Redis 密码必须被真实服务拒绝"
    );
    let session: Arc<dyn Session> = Arc::new(redis_session);
    RedisAgentSessionFactory::register_session(&bean_name, session.clone())
        .expect("应注册真实 Redis Session");

    let mut config = AgentConfig::default();
    config.session.memory.mode = MemoryStorageMode::Redis;
    config.session.memory.redis.bean_name = Some(bean_name.clone());
    config.session.memory.redis.key_prefix = key_prefix;
    let resolved = AgentSessionFactoryRegistry::new()
        .create_session(&config)
        .expect("工厂应解析真实 Redis Session")
        .expect("REDIS 模式应返回 Session");
    assert!(Arc::ptr_eq(&session, &resolved));

    let session_key = SessionKey::new("liteflow:redis-e2e");
    resolved.save_json_value(&session_key, "state", json!({"answer": 42}));
    resolved.save_list_json_values(
        &session_key,
        "messages",
        vec![json!("first"), json!({"role": "assistant"})],
    );

    let mut state = None;
    let mut messages = None;
    for _ in 0..40 {
        state = get_json_value(resolved.clone(), session_key.clone(), "state");
        messages = get_list_json_values(resolved.clone(), session_key.clone(), "messages");
        if state.is_some() && messages.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert_eq!(state, Some(json!({"answer": 42})));
    assert_eq!(
        messages,
        Some(vec![json!("first"), json!({"role": "assistant"})])
    );
    assert!(session_exists(resolved.clone(), session_key.clone()));

    resolved.delete(&session_key);
    for _ in 0..40 {
        if !session_exists(resolved.clone(), session_key.clone()) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(!session_exists(resolved, session_key));

    RedisAgentSessionFactory::unregister_session(&bean_name);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mysql_factory_drives_real_agentscope_session_persistence() {
    if !command_available("mysqld", "--version")
        || !command_available("mysql", "--version")
        || !command_available("mysqladmin", "--version")
    {
        eprintln!("mysqld/mysql/mysqladmin 不可用，跳过 Agent Session 真实 MySQL 测试");
        return;
    }

    // Anaconda MySQL 8.4 在 macOS 的 per-user /var/folders 临时目录中会错误地
    // 重建 undo tablespace；使用系统 /tmp 与命令行初始化/启动行为保持一致。
    let temp_dir = tempfile::Builder::new()
        .prefix("liteflow-mysql-")
        .tempdir_in("/tmp")
        .expect("应创建 MySQL 临时目录");
    initialize_mysql(temp_dir.path());
    let port = allocate_port();
    let _mysql_process = start_mysql(port, temp_dir.path());
    let database_name = format!("liteflow_agent_{port}");
    let table_name = format!("agent_sessions_{port}");
    let username = format!("liteflow_agent_{port}");
    prepare_mysql_database(port, &database_name, &username, MYSQL_PASSWORD).await;

    let mysql_url = format!("mysql://{username}:{MYSQL_PASSWORD}@127.0.0.1:{port}/{database_name}");
    let wrong_mysql_url =
        format!("mysql://{username}:wrong-password@127.0.0.1:{port}/{database_name}");
    assert!(
        MysqlSession::with_config(&wrong_mysql_url, &table_name, 1)
            .await
            .is_err(),
        "错误 MySQL 密码必须被真实服务拒绝"
    );
    let mysql_session = MysqlSession::with_config(&mysql_url, &table_name, 4)
        .await
        .expect("应连接真实 MySQL 并创建 Session 表");
    let session: Arc<dyn Session> = Arc::new(mysql_session);
    let data_source_bean_name = format!("agentMysqlSession{port}");
    MysqlAgentSessionFactory::register_session(&data_source_bean_name, session.clone())
        .expect("应注册真实 MySQL Session");

    let mut config = AgentConfig::default();
    config.session.memory.mode = MemoryStorageMode::Mysql;
    config.session.memory.mysql.data_source_bean_name = Some(data_source_bean_name.clone());
    config.session.memory.mysql.database_name = Some(database_name);
    config.session.memory.mysql.table_name = Some(table_name);
    config.session.memory.mysql.create_if_not_exist = true;
    let resolved = AgentSessionFactoryRegistry::new()
        .create_session(&config)
        .expect("工厂应解析真实 MySQL Session")
        .expect("MYSQL 模式应返回 Session");
    assert!(Arc::ptr_eq(&session, &resolved));

    let session_key = SessionKey::new("liteflow:mysql-e2e");
    resolved.save_json_value(&session_key, "state", json!({"answer": 84}));
    resolved.save_list_json_values(
        &session_key,
        "messages",
        vec![json!("first"), json!({"role": "tool"})],
    );

    let mut state = None;
    let mut messages = None;
    for _ in 0..80 {
        state = get_json_value(resolved.clone(), session_key.clone(), "state");
        messages = get_list_json_values(resolved.clone(), session_key.clone(), "messages");
        if state.is_some() && messages.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert_eq!(state, Some(json!({"answer": 84})));
    assert_eq!(
        messages,
        Some(vec![json!("first"), json!({"role": "tool"})])
    );
    assert!(session_exists(resolved.clone(), session_key.clone()));

    resolved.delete(&session_key);
    for _ in 0..80 {
        if !session_exists(resolved.clone(), session_key.clone()) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(!session_exists(resolved, session_key));

    MysqlAgentSessionFactory::unregister_session(&data_source_bean_name);
}
