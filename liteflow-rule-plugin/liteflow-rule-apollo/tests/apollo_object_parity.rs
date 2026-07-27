use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use liteflow_core::FlowBus;
use liteflow_core::parser::ParserClassNameSpi;
use liteflow_core::rule_plugin::{RuleSource, RuleSourceWatcher};
use liteflow_rule_apollo::parser::apollo::exception::ApolloException;
use liteflow_rule_apollo::parser::spi::apollo::ApolloParserClassNameSpi;
use liteflow_rule_apollo::{ApolloParserConfigVO, ApolloRuleSource};

struct ConfigServiceFixture {
    address: String,
    namespaces: Arc<RwLock<BTreeMap<String, BTreeMap<String, String>>>>,
    requests: Arc<RwLock<Vec<String>>>,
    stopped: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl ConfigServiceFixture {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("应能绑定本地 HTTP 端口");
        listener.set_nonblocking(true).expect("应能设置非阻塞监听");
        let address = listener.local_addr().expect("应能读取监听地址").to_string();
        let namespaces = Arc::new(RwLock::new(BTreeMap::new()));
        let requests = Arc::new(RwLock::new(Vec::new()));
        let stopped = Arc::new(AtomicBool::new(false));
        let server_namespaces = namespaces.clone();
        let server_requests = requests.clone();
        let server_stopped = stopped.clone();
        let thread = thread::spawn(move || {
            while !server_stopped.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        stream
                            .set_nonblocking(false)
                            .expect("测试 HTTP 连接应切换为阻塞读取");
                        handle_request(stream, &server_namespaces, &server_requests);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::yield_now();
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            address,
            namespaces,
            requests,
            stopped,
            thread: Some(thread),
        }
    }

    fn put(&self, namespace: &str, values: &[(&str, &str)]) {
        self.namespaces
            .write()
            .expect("namespace 写锁不应中毒")
            .insert(
                namespace.to_string(),
                values
                    .iter()
                    .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                    .collect(),
            );
    }

    fn url(&self) -> String {
        format!("http://{}", self.address)
    }

    fn requests(&self) -> Vec<String> {
        self.requests.read().expect("请求记录读锁不应中毒").clone()
    }
}

impl Drop for ConfigServiceFixture {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        let _ =
            TcpStream::connect(&self.address).and_then(|stream| stream.shutdown(Shutdown::Both));
        if let Some(thread) = self.thread.take() {
            thread.join().expect("本地 Config Service 应正常退出");
        }
    }
}

fn handle_request(
    mut stream: TcpStream,
    namespaces: &RwLock<BTreeMap<String, BTreeMap<String, String>>>,
    requests: &RwLock<Vec<String>>,
) {
    let mut buffer = [0_u8; 4096];
    let size = stream.read(&mut buffer).unwrap_or_default();
    if size == 0 {
        return;
    }
    let request = String::from_utf8_lossy(&buffer[..size]);
    let target = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");
    requests
        .write()
        .expect("请求记录写锁不应中毒")
        .push(target.to_string());
    let path = target.split('?').next().unwrap_or(target);
    let namespace = path.rsplit('/').next().unwrap_or_default();
    let body = namespaces
        .read()
        .expect("namespace 读锁不应中毒")
        .get(namespace)
        .cloned()
        .unwrap_or_default();
    let body = serde_json::to_string(&body).expect("测试配置应可序列化");
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream
        .write_all(response.as_bytes())
        .expect("应能写入测试响应");
}

#[test]
fn config_uses_camel_case_and_validates_required_namespace() {
    let config = ApolloParserConfigVO::new("chain-flow", Some("script-flow"));
    let json = serde_json::to_value(&config).expect("Apollo 配置应可序列化");
    assert_eq!(json["chainNamespace"], "chain-flow");
    assert_eq!(json["scriptNamespace"], "script-flow");

    let decoded: ApolloParserConfigVO =
        serde_json::from_value(json).expect("Apollo 配置应可反序列化");
    assert_eq!(decoded, config);

    let error = ApolloParserConfigVO::default()
        .validate()
        .expect_err("空 Chain namespace 必须失败");
    assert_eq!(
        error,
        ApolloException::new(
            "chainNamespace is empty, you must configure the chainNamespace property"
        )
    );
}

#[tokio::test]
async fn config_service_maps_are_aggregated_as_chain_and_script_xml() {
    let fixture = ConfigServiceFixture::start();
    fixture.put(
        "chain-flow",
        &[
            ("main:true", "THEN(script_node)"),
            ("offline:false", "THEN(x)"),
        ],
    );
    fixture.put(
        "script-flow",
        &[(
            "script_node:script:脚本节点:rhai:true",
            "let answer = 40 + 2; answer",
        )],
    );
    let source = ApolloRuleSource::new(fixture.url(), "sample", "default", "chain-flow")
        .expect("Apollo 规则源应创建成功")
        .with_script_namespace("script-flow")
        .expect("Script namespace 应设置成功")
        .with_ip("127.0.0.9");

    let (xml, fingerprint) = source.fetch().await.expect("应能读取本地 Config Service");
    assert!(xml.contains("<chain id=\"main\" enable=\"true\">THEN(script_node)</chain>"));
    assert!(xml.contains("<chain id=\"offline\" enable=\"false\">THEN(x)</chain>"));
    assert!(xml.contains("id=\"script_node\""));
    assert!(xml.contains("language=\"rhai\""));
    assert!(xml.contains("<![CDATA[let answer = 40 + 2; answer]]>"));
    assert!(!fingerprint.is_empty());

    let requests = fixture.requests();
    assert!(
        requests.contains(&"/configfiles/json/sample/default/chain-flow?ip=127.0.0.9".to_string())
    );
    assert!(
        requests.contains(&"/configfiles/json/sample/default/script-flow?ip=127.0.0.9".to_string())
    );
}

#[tokio::test]
async fn listener_reloads_changed_namespace_and_removes_deleted_chain() {
    let fixture = ConfigServiceFixture::start();
    fixture.put("chain-flow", &[("first:true", "THEN(script_node)")]);
    fixture.put(
        "script-flow",
        &[("script_node:script:script:rhai:true", "40 + 2")],
    );
    let source = ApolloRuleSource::new(fixture.url(), "sample", "default", "chain-flow")
        .expect("Apollo 规则源应创建成功")
        .with_script_namespace("script-flow")
        .expect("Script namespace 应设置成功");
    let helper = source.parser().helper().clone();
    let bus = FlowBus::new();
    let watcher = RuleSourceWatcher::new(bus.clone(), Arc::new(source))
        .await
        .expect("初始 Apollo 规则应装载成功");
    assert!(bus.contains_chain("first"));
    assert!(bus.contains_node("script_node"));
    let listener = helper.listen_apollo(watcher, Duration::from_millis(10));

    fixture.put("chain-flow", &[("second:true", "THEN(script_node)")]);
    for _ in 0..100 {
        if bus.contains_chain("second") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    listener.abort();
    assert!(!bus.contains_chain("first"));
    assert!(bus.contains_chain("second"));
}

#[test]
fn parser_spi_returns_java_aligned_class_name() {
    let spi = ApolloParserClassNameSpi;
    assert_eq!(
        spi.get_spi_class_name(),
        "com.yomahub.liteflow.parser.apollo.ApolloXmlELParser"
    );
}
