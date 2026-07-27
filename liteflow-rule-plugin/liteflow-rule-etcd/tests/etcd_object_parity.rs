use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use liteflow_core::FlowBus;
use liteflow_core::parser::ParserClassNameSpi;
use liteflow_core::rule_plugin::{RuleSource, RuleSourceWatcher};
use liteflow_rule_etcd::parser::spi::etcd::EtcdParserClassNameSpi;
use liteflow_rule_etcd::{EtcdClient, EtcdParserVO, EtcdRuleSource};

const ETCD_IMAGE: &str = "gcr.io/etcd-development/etcd:v3.5.21";

struct EtcdServer {
    container_name: String,
    endpoint: String,
}

impl EtcdServer {
    async fn start() -> Option<Self> {
        if !Command::new("docker")
            .args(["info", "--format", "{{.ServerVersion}}"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .ok()?
            .success()
        {
            eprintln!("Docker daemon 不可用，跳过真实 Etcd 测试");
            return None;
        }
        let listener = TcpListener::bind("127.0.0.1:0").ok()?;
        let port = listener.local_addr().ok()?.port();
        drop(listener);
        let container_name = format!("liteflow-etcd-test-{}-{port}", std::process::id());
        let mapping = format!("{port}:2379");
        let output = Command::new("docker")
            .args([
                "run",
                "-d",
                "--rm",
                "--name",
                &container_name,
                "-p",
                &mapping,
                ETCD_IMAGE,
                "/usr/local/bin/etcd",
                "--name",
                "s1",
                "--data-dir",
                "/tmp/etcd-data",
                "--listen-client-urls",
                "http://0.0.0.0:2379",
                "--advertise-client-urls",
                "http://0.0.0.0:2379",
                "--listen-peer-urls",
                "http://0.0.0.0:2380",
                "--initial-advertise-peer-urls",
                "http://0.0.0.0:2380",
                "--initial-cluster",
                "s1=http://0.0.0.0:2380",
                "--initial-cluster-token",
                "liteflow-test",
                "--initial-cluster-state",
                "new",
                "--log-level",
                "warn",
            ])
            .output()
            .ok()?;
        if !output.status.success() {
            eprintln!(
                "Etcd 容器启动失败: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            return None;
        }
        let endpoint = format!("http://127.0.0.1:{port}");
        let server = Self {
            container_name,
            endpoint,
        };
        let client = EtcdClient::new(vec![server.endpoint.clone()], None, None, None);
        for _ in 0..100 {
            if client.put("/health", "ready").await.is_ok() {
                return Some(server);
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        None
    }
}

impl Drop for EtcdServer {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["rm", "-f", &self.container_name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

#[test]
fn config_uses_camel_case_and_validates_required_fields() {
    let mut config = EtcdParserVO::new(
        "http://127.0.0.1:2379, http://127.0.0.1:22379",
        "/liteflow/chain",
    );
    config.set_user(Some("root"));
    config.set_password(Some("secret"));
    config.set_namespace(Some("/tenant"));
    config.set_script_path(Some("/liteflow/script"));
    let json = serde_json::to_value(&config).expect("Etcd 配置应可序列化");
    assert_eq!(json["chainPath"], "/liteflow/chain");
    assert_eq!(json["scriptPath"], "/liteflow/script");
    assert_eq!(config.endpoint_list().len(), 2);
    let decoded: EtcdParserVO = serde_json::from_value(json).expect("Etcd 配置应可反序列化");
    assert_eq!(decoded, config);

    assert_eq!(
        EtcdParserVO::default()
            .validate()
            .expect_err("空 chainPath 必须失败")
            .to_string(),
        "You must configure the chainPath property"
    );
    assert_eq!(
        EtcdParserVO::new("", "/chain")
            .validate()
            .expect_err("空 endpoints 必须失败")
            .to_string(),
        "etcd endpoints is empty"
    );
}

#[tokio::test]
async fn real_etcd_prefix_aggregation_namespace_and_watch_are_executed() {
    let Some(server) = EtcdServer::start().await else {
        return;
    };
    let admin = EtcdClient::new(vec![server.endpoint.clone()], None, None, None);
    admin
        .put("/liteflow/chain/first:true", "THEN(script_node)")
        .await
        .expect("应能写入初始 Chain");
    admin
        .put(
            "/liteflow/script/script_node:script:script:rhai:true",
            "40 + 2",
        )
        .await
        .expect("应能写入脚本");

    let children = admin
        .get_children_keys("/liteflow/chain", "/")
        .await
        .expect("应能执行前缀查询");
    assert_eq!(children, vec!["first:true".to_string()]);
    let previous = admin
        .put("/liteflow/chain/first:true", "THEN(script_node)")
        .await
        .expect("应能覆盖 Chain");
    assert_eq!(previous.as_deref(), Some("THEN(script_node)"));

    let (event_sender, mut event_receiver) = tokio::sync::mpsc::channel(2);
    admin
        .watch_data_change(
            "/single-watch",
            move |path, value| {
                let _ = event_sender.try_send((path, value));
            },
            |_| {},
        )
        .await
        .expect("单 key Watch 应安装成功");
    admin
        .put("/single-watch", "first")
        .await
        .expect("应能触发单 key Watch");
    let event = tokio::time::timeout(Duration::from_secs(2), event_receiver.recv())
        .await
        .expect("应在超时前收到单 key Watch 事件")
        .expect("Watch channel 不应关闭");
    assert_eq!(event, ("/single-watch".to_string(), "first".to_string()));
    admin.watch_close("/single-watch").await;
    admin
        .put("/single-watch", "second")
        .await
        .expect("取消 Watch 后写入仍应成功");
    let after_close = tokio::time::timeout(Duration::from_millis(150), event_receiver.recv()).await;
    assert!(
        !matches!(after_close, Ok(Some(_))),
        "watchClose 后不应再收到事件"
    );

    let namespaced = EtcdClient::new(
        vec![server.endpoint.clone()],
        Some("/tenant".to_string()),
        None,
        None,
    );
    namespaced
        .put("/probe", "namespaced")
        .await
        .expect("namespace 写入应成功");
    assert_eq!(
        namespaced
            .get("/probe")
            .await
            .expect("namespace 读取应成功"),
        Some("namespaced".to_string())
    );
    let mut raw = etcd_client::Client::connect([server.endpoint.clone()], None)
        .await
        .expect("原始 Etcd 客户端应连接成功");
    assert_eq!(
        raw.get("/tenant/probe", None)
            .await
            .expect("应能读取物理 namespace key")
            .kvs()
            .first()
            .expect("物理 namespace key 应存在")
            .value_str()
            .expect("值应为 UTF-8"),
        "namespaced"
    );

    let source = EtcdRuleSource::new(vec![server.endpoint.clone()], "/liteflow/chain")
        .expect("Etcd 规则源配置应有效")
        .with_script_path("/liteflow/script")
        .expect("Script 路径应有效");
    let (xml, fingerprint) = source.fetch().await.expect("应能聚合 Etcd 前缀树");
    assert!(xml.contains("<chain id=\"first\" enable=\"true\">THEN(script_node)</chain>"));
    assert!(xml.contains("id=\"script_node\""));
    assert!(xml.contains("language=\"rhai\""));
    assert!(!fingerprint.is_empty());

    let parser = source.parser().clone();
    let bus = FlowBus::new();
    let watcher = RuleSourceWatcher::new(bus.clone(), Arc::new(source))
        .await
        .expect("初始 Etcd 规则应装载成功");
    parser.listen(watcher).await.expect("Etcd Watch 应安装成功");
    assert!(bus.contains_chain("first"));
    assert!(bus.contains_node("script_node"));

    raw.delete("/liteflow/chain/first:true", None)
        .await
        .expect("应能删除旧 Chain");
    raw.put("/liteflow/chain/second:true", "THEN(script_node)", None)
        .await
        .expect("应能新增 Chain");
    for _ in 0..100 {
        if bus.contains_chain("second") && !bus.contains_chain("first") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(bus.contains_chain("second"));
    assert!(!bus.contains_chain("first"));

    raw.delete("/liteflow/script/script_node:script:script:rhai:true", None)
        .await
        .expect("应能删除脚本节点");
    for _ in 0..100 {
        if !bus.contains_node("script_node") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(!bus.contains_node("script_node"));
    parser.helper().client().close().await;
    admin.close().await;
    namespaced.close().await;
}

#[test]
fn parser_spi_returns_java_aligned_class_name() {
    let spi = EtcdParserClassNameSpi;
    assert_eq!(
        spi.get_spi_class_name(),
        "com.yomahub.liteflow.parser.etcd.EtcdXmlELParser"
    );
}
