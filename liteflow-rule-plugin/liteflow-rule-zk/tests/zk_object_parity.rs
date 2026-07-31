use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use liteflow_core::FlowBus;
use liteflow_core::parser::ParserClassNameSpi;
use liteflow_core::rule_plugin::{RuleSource, RuleSourceWatcher};
use liteflow_rule_zk::parser::spi::zk::ZkParserClassNameSpi;
use liteflow_rule_zk::{ZkParserVO, ZkRuleSource};
use zookeeper_client::{Acls, Client, CreateMode};

const ZOOKEEPER_IMAGE: &str = "zookeeper:3.9.3";

struct ZkServer {
    container_name: String,
    connect_str: String,
}

impl ZkServer {
    async fn start() -> Option<Self> {
        if !Command::new("docker")
            .args(["info", "--format", "{{.ServerVersion}}"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .ok()?
            .success()
        {
            eprintln!("Docker daemon 不可用，跳过真实 ZooKeeper 测试");
            return None;
        }
        let listener = TcpListener::bind("127.0.0.1:0").ok()?;
        let port = listener.local_addr().ok()?.port();
        drop(listener);
        let container_name = format!("liteflow-zk-test-{}-{port}", std::process::id());
        let mapping = format!("{port}:2181");
        let output = Command::new("docker")
            .args([
                "run",
                "-d",
                "--rm",
                "--name",
                &container_name,
                "-p",
                &mapping,
                ZOOKEEPER_IMAGE,
            ])
            .output()
            .ok()?;
        if !output.status.success() {
            eprintln!(
                "ZooKeeper 容器启动失败: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            return None;
        }
        let server = Self {
            container_name,
            connect_str: format!("127.0.0.1:{port}"),
        };
        for _ in 0..120 {
            if let Ok(client) = Client::connector()
                .with_session_timeout(Duration::from_secs(5))
                .with_connection_timeout(Duration::from_secs(3))
                .connect(&server.connect_str)
                .await
                && client.get_children("/").await.is_ok()
            {
                drop(client);
                return Some(server);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        None
    }
}

impl Drop for ZkServer {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["rm", "-f", &self.container_name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

async fn create_node(client: &Client, path: &str, data: &str) {
    client
        .create(
            path,
            data.as_bytes(),
            &CreateMode::Persistent.with_acls(Acls::anyone_all()),
        )
        .await
        .expect("应能创建 ZooKeeper 测试节点");
}

#[test]
fn config_uses_camel_case_and_validates_required_fields() {
    let mut config = ZkParserVO::new("127.0.0.1:2181", "/liteflow/chain");
    config.set_script_path(Some("/liteflow/script"));
    let json = serde_json::to_value(&config).expect("ZooKeeper 配置应可序列化");
    assert_eq!(json["connectStr"], "127.0.0.1:2181");
    assert_eq!(json["chainPath"], "/liteflow/chain");
    assert_eq!(json["scriptPath"], "/liteflow/script");
    let decoded: ZkParserVO = serde_json::from_value(json).expect("ZooKeeper 配置应可反序列化");
    assert_eq!(decoded, config);

    assert_eq!(
        ZkParserVO::default()
            .validate()
            .expect_err("空 Chain 路径必须失败")
            .to_string(),
        "You must configure the chainPath property"
    );
    assert_eq!(
        ZkParserVO::new("", "/chain")
            .validate()
            .expect_err("空连接串必须失败")
            .to_string(),
        "zk connect string is empty"
    );
}

#[tokio::test]
async fn real_zookeeper_child_aggregation_and_persistent_watch_are_executed() {
    let Some(server) = ZkServer::start().await else {
        return;
    };
    let admin = Client::connect(&server.connect_str)
        .await
        .expect("应能连接真实 ZooKeeper");
    admin
        .mkdir(
            "/liteflow/chain",
            &CreateMode::Persistent.with_acls(Acls::anyone_all()),
        )
        .await
        .expect("应能创建 Chain 根路径");
    admin
        .mkdir(
            "/liteflow/script",
            &CreateMode::Persistent.with_acls(Acls::anyone_all()),
        )
        .await
        .expect("应能创建 Script 根路径");
    create_node(&admin, "/liteflow/chain/first:true", "THEN(script_node)").await;
    create_node(
        &admin,
        "/liteflow/script/script_node:script:script:rhai:true",
        "40 + 2",
    )
    .await;

    let source = ZkRuleSource::new(&server.connect_str, "/liteflow/chain")
        .expect("ZooKeeper 规则源配置应有效")
        .with_script_path("/liteflow/script")
        .expect("Script 路径配置应有效");
    let (xml, fingerprint) = source.fetch().await.expect("应能聚合真实 ZooKeeper 子节点");
    assert!(xml.contains("<chain id=\"first\" enable=\"true\">THEN(script_node)</chain>"));
    assert!(xml.contains("id=\"script_node\""));
    assert!(xml.contains("language=\"rhai\""));
    assert!(!fingerprint.is_empty());

    let parser = source.parser().clone();
    let bus = FlowBus::new();
    let watcher = RuleSourceWatcher::new(bus.clone(), Arc::new(source))
        .await
        .expect("初始 ZooKeeper 规则应装载成功");
    parser
        .listen(watcher)
        .await
        .expect("ZooKeeper 持久递归 Watch 应安装成功");
    assert!(bus.contains_chain("first"));
    assert!(bus.contains_node("script_node"));

    admin
        .delete("/liteflow/chain/first:true", None)
        .await
        .expect("应能删除旧 Chain");
    create_node(&admin, "/liteflow/chain/second:true", "THEN(script_node)").await;
    for _ in 0..150 {
        if bus.contains_chain("second") && !bus.contains_chain("first") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(bus.contains_chain("second"));
    assert!(!bus.contains_chain("first"));

    admin
        .delete("/liteflow/script/script_node:script:script:rhai:true", None)
        .await
        .expect("应能删除 Script 节点");
    for _ in 0..100 {
        if !bus.contains_node("script_node") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(!bus.contains_node("script_node"));

    drop(admin);
}

#[test]
fn parser_spi_returns_java_aligned_class_name() {
    let spi = ZkParserClassNameSpi;
    assert_eq!(
        spi.get_spi_class_name(),
        "com.yomahub.liteflow.parser.zk.ZkXmlELParser"
    );
}
