use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use async_trait::async_trait;
use liteflow_core::cmp;
use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::flow::FlowBus;
use liteflow_core::rule_plugin::{RuleFormat, RuleSource, RuleSourceWatcher, fnv_fp};
use serde_json::Value;

#[derive(Clone)]
struct MutableRuleSource {
    name: String,
    format: RuleFormat,
    state: Arc<RwLock<SourceState>>,
    fetch_count: Arc<AtomicUsize>,
}

#[derive(Clone)]
struct SourceState {
    text: String,
    fingerprint: String,
    fail: bool,
}

impl MutableRuleSource {
    fn new(name: &str, format: RuleFormat, text: &str, fingerprint: &str) -> Self {
        Self {
            name: name.to_string(),
            format,
            state: Arc::new(RwLock::new(SourceState {
                text: text.to_string(),
                fingerprint: fingerprint.to_string(),
                fail: false,
            })),
            fetch_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn publish(&self, text: &str, fingerprint: &str) {
        let mut state = self.state.write().expect("测试规则源写锁不应中毒");
        state.text = text.to_string();
        state.fingerprint = fingerprint.to_string();
        state.fail = false;
    }

    fn set_failure(&self, fail: bool) {
        self.state.write().expect("测试规则源写锁不应中毒").fail = fail;
    }

    fn fetch_count(&self) -> usize {
        self.fetch_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl RuleSource for MutableRuleSource {
    async fn fetch(&self) -> LFResult<(String, String)> {
        self.fetch_count.fetch_add(1, Ordering::SeqCst);
        let state = self.state.read().expect("测试规则源读锁不应中毒").clone();
        if state.fail {
            return Err(LiteflowError::Rule("模拟规则源读取失败".to_string()));
        }
        Ok((state.text, state.fingerprint))
    }

    fn format(&self) -> RuleFormat {
        self.format
    }

    fn name(&self) -> &str {
        &self.name
    }
}

fn bus_with_node() -> FlowBus {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    bus
}

#[test]
fn fnv_fingerprint_is_stable_for_utf8_bytes() {
    assert_eq!(fnv_fp(""), "cbf29ce484222325");
    assert_eq!(fnv_fp("a"), "af63dc4c8601ec8c");
    assert_eq!(fnv_fp("规则"), fnv_fp("规则"));
    assert_ne!(fnv_fp("规则"), fnv_fp("规则 "));
}

#[tokio::test]
async fn initial_load_dispatches_json_xml_and_yml_formats() {
    let cases = [
        (
            RuleFormat::Json,
            r#"{"flow":{"chain":[{"id":"json_chain","body":"THEN(a)"}]}}"#,
            "json_chain",
        ),
        (
            RuleFormat::Xml,
            r#"<flow><chain id="xml_chain"><body>THEN(a)</body></chain></flow>"#,
            "xml_chain",
        ),
        (
            RuleFormat::Yml,
            "flow:\n  chain:\n    - id: yml_chain\n      body: THEN(a)\n",
            "yml_chain",
        ),
    ];

    for (format, text, chain_id) in cases {
        let bus = bus_with_node();
        let source = Arc::new(MutableRuleSource::new("fixture", format, text, "v1"));
        let _watcher = RuleSourceWatcher::new(bus.clone(), source)
            .await
            .expect("三种规则格式都应完成首次装载");
        assert!(bus.contains_chain(chain_id));
    }
}

#[tokio::test]
async fn reload_reconciles_removed_chains_and_preserves_state_on_parse_failure() {
    let old_rule = r#"{"flow":{"chain":[{"id":"old_chain","body":"THEN(a)"}]}}"#;
    let new_rule = r#"{"flow":{"chain":[{"id":"new_chain","body":"THEN(a)"}]}}"#;
    let bus = bus_with_node();
    let source = Arc::new(MutableRuleSource::new(
        "reload-fixture",
        RuleFormat::Json,
        old_rule,
        "v1",
    ));
    let watcher = RuleSourceWatcher::new(bus.clone(), source.clone())
        .await
        .expect("初始规则应装载成功");

    source.publish(new_rule, "v2");
    assert_eq!(
        watcher.reload().await.expect("合法新规则应热更新"),
        vec!["new_chain"]
    );
    assert!(!bus.contains_chain("old_chain"));
    assert!(bus.contains_chain("new_chain"));

    bus.register("temporary_script", cmp(|_| async { Ok(Value::Null) }));
    assert!(bus.contains_node("temporary_script"));
    watcher.unload_script_node("temporary_script");
    assert!(!bus.contains_node("temporary_script"));

    source.publish("{", "broken");
    assert!(watcher.reload().await.is_err());
    assert!(bus.contains_chain("new_chain"));
}

#[tokio::test]
async fn watch_ignores_equal_fingerprints_retries_failures_and_publishes_next_valid_rule() {
    let initial_rule = r#"{"flow":{"chain":[{"id":"initial","body":"THEN(a)"}]}}"#;
    let final_rule = r#"{"flow":{"chain":[{"id":"final_chain","body":"THEN(a)"}]}}"#;
    let bus = bus_with_node();
    let source = Arc::new(MutableRuleSource::new(
        "watch-fixture",
        RuleFormat::Json,
        initial_rule,
        "v1",
    ));
    let watcher = RuleSourceWatcher::new(bus.clone(), source.clone())
        .await
        .expect("初始规则应装载成功");
    let task = watcher.watch(Duration::from_millis(5));

    // 相同指纹即使被多次拉取也不得重复发布。
    tokio::time::sleep(Duration::from_millis(25)).await;
    assert!(source.fetch_count() > 1);
    assert!(bus.contains_chain("initial"));

    // 拉取失败和新版本解析失败都保留最后一次成功快照，后续轮询继续重试。
    source.set_failure(true);
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(bus.contains_chain("initial"));

    source.publish("{", "v2");
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(bus.contains_chain("initial"));

    source.publish(final_rule, "v3");
    for _ in 0..40 {
        if bus.contains_chain("final_chain") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(bus.contains_chain("final_chain"));
    assert!(!bus.contains_chain("initial"));

    task.abort();
    assert!(
        task.await
            .expect_err("abort 后监听任务应取消")
            .is_cancelled()
    );
}
