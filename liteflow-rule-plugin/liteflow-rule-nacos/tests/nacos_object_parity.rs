use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use liteflow_core::FlowBus;
use liteflow_core::parser::ParserClassNameSpi;
use liteflow_core::rule_plugin::{RuleSource, RuleSourceWatcher};
use liteflow_rule_nacos::parser::nacos::exception::NacosException;
use liteflow_rule_nacos::parser::spi::nacos::NacosParserClassNameSpi;
use liteflow_rule_nacos::{NacosParserVO, NacosRuleSource};
use nacos_sdk::api::config::{ConfigService, ConfigServiceBuilder};
use nacos_sdk::api::props::ClientProps;

const NACOS_IMAGE: &str = "nacos/nacos-server:v2.5.1-slim";

struct NacosServer {
    container_name: String,
    server_addr: String,
}

impl NacosServer {
    async fn start() -> Option<Self> {
        if !Command::new("docker")
            .args(["info", "--format", "{{.ServerVersion}}"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .ok()?
            .success()
        {
            eprintln!("Docker daemon 不可用，跳过真实 Nacos 测试");
            return None;
        }

        let (http_port, grpc_port) = reserve_nacos_ports()?;
        let container_name = format!("liteflow-nacos-test-{}-{http_port}", std::process::id());
        let http_mapping = format!("{http_port}:8848");
        let grpc_mapping = format!("{grpc_port}:9848");
        let output = Command::new("docker")
            .args([
                "run",
                "-d",
                "--rm",
                "--name",
                &container_name,
                "-e",
                "MODE=standalone",
                "-e",
                "NACOS_AUTH_ENABLE=false",
                "-e",
                "JVM_XMS=256m",
                "-e",
                "JVM_XMX=256m",
                "-e",
                "JVM_XMN=128m",
                "-p",
                &http_mapping,
                "-p",
                &grpc_mapping,
                NACOS_IMAGE,
            ])
            .output()
            .ok()?;
        if !output.status.success() {
            eprintln!(
                "Nacos 容器启动失败: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            return None;
        }

        let server = Self {
            container_name,
            server_addr: format!("127.0.0.1:{http_port}"),
        };
        for _ in 0..180 {
            if TcpStream::connect(&server.server_addr).is_ok() {
                tokio::time::sleep(Duration::from_millis(500)).await;
                return Some(server);
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        eprintln!("Nacos 容器未在等待窗口内就绪");
        None
    }

    async fn client(&self) -> ConfigService {
        for _ in 0..120 {
            let result = ConfigServiceBuilder::new(
                ClientProps::new()
                    .server_addr(&self.server_addr)
                    .namespace("")
                    .app_name("liteflow-rust-test")
                    .env_first(false),
            )
            .build()
            .await;
            if let Ok(service) = result {
                return service;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        panic!("应能连接真实 Nacos 服务");
    }
}

impl Drop for NacosServer {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["rm", "-f", &self.container_name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

fn reserve_nacos_ports() -> Option<(u16, u16)> {
    for _ in 0..100 {
        let http = TcpListener::bind("127.0.0.1:0").ok()?;
        let http_port = http.local_addr().ok()?.port();
        let Some(grpc_port) = http_port.checked_add(1000) else {
            continue;
        };
        if let Ok(grpc) = TcpListener::bind(("127.0.0.1", grpc_port)) {
            drop(grpc);
            drop(http);
            return Some((http_port, grpc_port));
        }
    }
    None
}

#[test]
fn config_defaults_camel_case_auth_pairs_and_content_validation() {
    let config = NacosParserVO::default();
    assert_eq!(config.server_addr(), "127.0.0.1:8848");
    assert_eq!(config.namespace(), "");
    assert_eq!(config.data_id(), "LiteFlow");
    assert_eq!(config.group(), "LITE_FLOW_GROUP");

    let json = serde_json::to_value(&config).expect("Nacos 配置应可序列化");
    assert_eq!(json["serverAddr"], "127.0.0.1:8848");
    assert_eq!(json["dataId"], "LiteFlow");
    assert_eq!(json["accessKey"], "");
    let decoded: NacosParserVO = serde_json::from_value(json).expect("Nacos 配置应可反序列化");
    assert_eq!(decoded, config);

    let mut invalid_auth = NacosParserVO::default();
    invalid_auth.set_username("nacos");
    assert_eq!(
        invalid_auth
            .validate()
            .expect_err("用户名密码必须成对")
            .to_string(),
        "username and password must be configured together"
    );
    let mut invalid_access_key = NacosParserVO::default();
    invalid_access_key.set_access_key("ak");
    assert_eq!(
        invalid_access_key
            .validate()
            .expect_err("AccessKey 与 SecretKey 必须成对")
            .to_string(),
        "accessKey and secretKey must be configured together"
    );

    let source = NacosRuleSource::from_config(config).expect("默认 Nacos 配置应有效");
    assert_eq!(
        source
            .parser()
            .helper()
            .check_content("  ")
            .expect_err("空白规则必须失败"),
        NacosException::new("the node[LiteFlow] value is empty")
    );
}

#[tokio::test]
async fn real_nacos_publish_fetch_and_native_listener_are_executed() {
    let Some(server) = NacosServer::start().await else {
        return;
    };
    let admin = server.client().await;
    let data_id = format!("liteflow-rust-{}", std::process::id());
    let group = "LITE_FLOW_GROUP".to_string();
    let first_xml =
        r#"<?xml version="1.0" encoding="UTF-8"?><flow><chain id="first">THEN(a)</chain></flow>"#;
    assert!(
        admin
            .publish_config(
                data_id.clone(),
                group.clone(),
                first_xml.to_string(),
                Some("xml".to_string()),
            )
            .await
            .expect("应能向真实 Nacos 发布初始规则")
    );
    for _ in 0..100 {
        if admin
            .get_config(data_id.clone(), group.clone())
            .await
            .is_ok_and(|response| response.content() == first_xml)
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let source = NacosRuleSource::new(&server.server_addr, &data_id, &group)
        .expect("Nacos 规则源配置应有效");
    let (fetched, fingerprint) = source.fetch().await.expect("应能读取真实 Nacos 配置");
    assert_eq!(fetched, first_xml);
    assert!(!fingerprint.is_empty());

    let parser = source.parser().clone();
    let bus = FlowBus::new();
    let watcher = RuleSourceWatcher::new(bus.clone(), Arc::new(source))
        .await
        .expect("初始 Nacos 规则应装载成功");
    assert!(bus.contains_chain("first"));
    parser
        .listen(watcher)
        .await
        .expect("Nacos 原生 Listener 应安装成功");

    let second_xml =
        r#"<?xml version="1.0" encoding="UTF-8"?><flow><chain id="second">THEN(b)</chain></flow>"#;
    assert!(
        admin
            .publish_config(
                data_id.clone(),
                group.clone(),
                second_xml.to_string(),
                Some("xml".to_string()),
            )
            .await
            .expect("应能发布变更后的规则")
    );
    for _ in 0..150 {
        if bus.contains_chain("second") && !bus.contains_chain("first") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(bus.contains_chain("second"));
    assert!(!bus.contains_chain("first"));

    assert!(
        admin
            .remove_config(data_id, group)
            .await
            .expect("应能清理真实 Nacos 测试配置")
    );
}

#[test]
fn parser_spi_returns_java_aligned_class_name() {
    let spi = NacosParserClassNameSpi;
    assert_eq!(
        spi.get_spi_class_name(),
        "com.yomahub.liteflow.parser.nacos.NacosXmlELParser"
    );
}
