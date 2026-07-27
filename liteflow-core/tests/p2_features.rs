//! P2 迁移项测试：YML 规则 / 链继承 / 子链嵌套 / 声明式组件 / AOP / 监控 / 生命周期 / 实例编号。

use liteflow_core::{CmpContext, FlowBus, LiteflowError, MonitorFile, MonitorTimeTask, cmp, rule};
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

fn null_ok() -> Result<Value, LiteflowError> {
    Ok(Value::Null)
}

#[tokio::test]
async fn monitor_file_manages_real_reload_delete_and_destroy_lifecycle() {
    let directory = tempfile::tempdir().unwrap();
    let rule_file = directory.path().join("flow.json");
    std::fs::write(
        &rule_file,
        r#"{"flow":{"chain":[{"name":"watched_a","condition":[{"type":"then","value":"a"}]}]}}"#,
    )
    .unwrap();

    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    let monitor = MonitorFile::new(bus.clone());
    monitor.add_monitor_file_path(&rule_file).unwrap();
    monitor.create(Duration::from_millis(10)).unwrap();
    assert!(monitor.is_monitoring());
    assert!(bus.contains_chain("watched_a"));

    tokio::time::sleep(Duration::from_millis(20)).await;
    std::fs::write(
        &rule_file,
        r#"{"flow":{"chain":[{"name":"watched_b","condition":[{"type":"then","value":"a"}]}]}}"#,
    )
    .unwrap();
    for _ in 0..50 {
        if bus.contains_chain("watched_b") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(!bus.contains_chain("watched_a"));
    assert!(bus.contains_chain("watched_b"));

    std::fs::remove_file(&rule_file).unwrap();
    for _ in 0..50 {
        if !bus.contains_chain("watched_b") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(!bus.contains_chain("watched_b"));

    monitor.destroy().unwrap();
    assert!(!monitor.is_monitoring());
}

#[tokio::test]
async fn flow_bus_clean_monitor_file_stops_registered_monitor_tasks() {
    let directory = tempfile::tempdir().unwrap();
    let rule_file = directory.path().join("flow.json");
    std::fs::write(
        &rule_file,
        r#"{"flow":{"chain":[{"name":"watched","condition":[{"type":"then","value":"a"}]}]}}"#,
    )
    .unwrap();

    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    let monitor = MonitorFile::new(bus.clone());
    monitor.add_monitor_file_path(&rule_file).unwrap();
    monitor.create(Duration::from_millis(10)).unwrap();
    assert!(monitor.is_monitoring());

    bus.clean_monitor_file().unwrap();
    assert!(!monitor.is_monitoring());
}

#[test]
fn monitor_file_java_events_share_runtime_instance_and_real_chain_state() {
    let directory = tempfile::tempdir().unwrap();
    let rule_file = directory.path().join("events.json");
    std::fs::write(
        &rule_file,
        r#"{"flow":{"chain":[{"name":"event_a","condition":[{"type":"then","value":"a"}]}]}}"#,
    )
    .unwrap();

    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    let monitor = MonitorFile::get_instance(bus.clone());
    let same_monitor = MonitorFile::get_instance(bus.clone());
    let other_monitor = MonitorFile::get_instance(FlowBus::new());
    assert!(Arc::ptr_eq(&monitor, &same_monitor));
    assert!(!Arc::ptr_eq(&monitor, &other_monitor));

    monitor.on_file_create(&rule_file).unwrap();
    assert!(bus.contains_chain("event_a"));

    std::fs::write(
        &rule_file,
        r#"{"flow":{"chain":[{"name":"event_b","condition":[{"type":"then","value":"a"}]}]}}"#,
    )
    .unwrap();
    monitor.on_file_change(&rule_file).unwrap();
    assert!(!bus.contains_chain("event_a"));
    assert!(bus.contains_chain("event_b"));

    // 坏规则不能删除上一版可执行 Chain，验证热更新的失败保留语义。
    std::fs::write(&rule_file, "{broken json").unwrap();
    assert!(monitor.on_file_change(&rule_file).is_err());
    assert!(bus.contains_chain("event_b"));

    std::fs::remove_file(&rule_file).unwrap();
    monitor.on_file_delete(&rule_file).unwrap();
    assert!(!bus.contains_chain("event_b"));
}

// ---------- YML 规则 ----------

#[tokio::test]
async fn yml_rule_loading() {
    let bus = FlowBus::new();
    bus.register(
        "a",
        cmp(|ctx| async move {
            ctx.set_data("r", json!("ran"));
            Ok(Value::Null)
        }),
    );
    let yml = r#"
flow:
  chain:
    - name: chainY
      condition:
        - type: then
          value: a
"#;
    let ids = rule::load_yml_str(&bus, yml).unwrap();
    assert_eq!(ids, vec!["chainY"]);
    assert_eq!(bus.execute("chainY").await.data("r"), Some(json!("ran")));
}

#[tokio::test]
async fn yml_route_chain() {
    let bus = FlowBus::new();
    bus.register("r1", cmp(|_| async { Ok(json!(true)) }));
    bus.register(
        "a",
        cmp(|ctx| async move {
            ctx.set_data("hit", json!(true));
            Ok(Value::Null)
        }),
    );
    let yml = r#"
flow:
  chain:
    - id: rc
      namespace: ns1
      route: r1
      body: THEN(a)
"#;
    rule::load_yml_str(&bus, yml).unwrap();
    let resps = bus
        .execute_route_chain(Some("ns1"), Value::Null)
        .await
        .unwrap();
    assert_eq!(resps.len(), 1);
}

// ---------- 链继承（extends + {{占位符}}） ----------

#[tokio::test]
async fn chain_inheritance() {
    let bus = FlowBus::new();
    for id in ["a", "b", "c", "d"] {
        bus.register(
            id,
            cmp(|ctx| async move {
                let mut v: Vec<String> = ctx.get_data_as("seq").unwrap_or_default();
                v.push(ctx.node_id().to_string());
                ctx.set_data("seq", json!(v));
                Ok(Value::Null)
            }),
        );
    }
    let json_rule = r#"{"flow":{"chain":[
        {"id":"parent", "body":"THEN(a, {{x}}, WHEN({{y}}, d))"},
        {"id":"child", "extends":"parent", "body":"{{x}} = b; {{y}} = c;"}
    ]}}"#;
    let ids = rule::load_json_str(&bus, json_rule).unwrap();
    assert_eq!(ids, vec!["child"]);
    assert!(!bus.contains_chain("parent"));
    let resp = bus.execute("child").await;
    assert!(resp.is_success());
    let seq: Vec<String> = resp.data_as("seq").unwrap();
    assert_eq!(seq[0], "a");
    assert_eq!(seq[1], "b");
    assert!(seq.contains(&"c".to_string()));
    assert!(seq.contains(&"d".to_string()));
}

#[tokio::test]
async fn chain_inheritance_missing_impl() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { null_ok() }));
    let json_rule = r#"{"flow":{"chain":[
        {"id":"parent", "body":"THEN(a, {{x}})"},
        {"id":"child", "extends":"parent", "body":"THEN(b)"}
    ]}}"#;
    assert!(rule::load_json_str(&bus, json_rule).is_err());
}

// ---------- 子链嵌套（ChainBindWrapperCondition） ----------

#[tokio::test]
async fn sub_chain_reference() {
    let bus = FlowBus::new();
    for id in ["a", "b", "c"] {
        bus.register(
            id,
            cmp(|ctx| async move {
                let mut seq: Vec<String> = ctx.get_data_as("seq").unwrap_or_default();
                seq.push(ctx.node_id().to_string());
                ctx.set_data("seq", json!(seq));
                Ok(Value::Null)
            }),
        );
    }
    bus.add_chain("subChain", "THEN(b)").unwrap();
    bus.add_chain("mainChain", "THEN(a, subChain, c)").unwrap();
    let resp = bus.execute("mainChain").await;
    assert!(resp.is_success());
    let seq: Vec<String> = resp.data_as("seq").unwrap();
    assert_eq!(seq, vec!["a", "b", "c"]);
}

// ---------- 声明式组件（@LiteflowMethod 语义） ----------

struct OrderCmp;

#[async_trait::async_trait]
impl liteflow_core::core::decl_component::DeclComponent for OrderCmp {
    async fn call(&self, method: &str, ctx: &CmpContext) -> Result<Value, LiteflowError> {
        match method {
            "checkStock" => {
                ctx.set_data("stock", json!(true));
                Ok(Value::Null)
            }
            "isVip" => Ok(json!(true)),
            other => Err(LiteflowError::Custom(format!("unknown method: {other}"))),
        }
    }
}

#[tokio::test]
async fn decl_component_method() {
    let bus = FlowBus::new();
    bus.register_decl("orderCmp", Arc::new(OrderCmp));
    bus.register(
        "vip",
        cmp(|ctx| async move {
            ctx.set_data("plan", json!("vip"));
            Ok(Value::Null)
        }),
    );
    bus.register(
        "normal",
        cmp(|ctx| async move {
            ctx.set_data("plan", json!("normal"));
            Ok(Value::Null)
        }),
    );
    bus.add_chain(
        "c1",
        "THEN(orderCmp.checkStock, IF(orderCmp.isVip, vip, normal))",
    )
    .unwrap();
    let resp = bus.execute("c1").await;
    assert!(resp.is_success());
    assert_eq!(resp.data("stock"), Some(json!(true)));
    assert_eq!(resp.data("plan"), Some(json!("vip")));
}

// ---------- 全局 AOP ----------

struct CountAspect {
    before: Arc<AtomicUsize>,
    after: Arc<AtomicUsize>,
    errors: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl liteflow_core::aop::ICmpAroundAspect for CountAspect {
    async fn before_process(&self, _ctx: &CmpContext) {
        self.before.fetch_add(1, Ordering::SeqCst);
    }
    async fn after_process(&self, _ctx: &CmpContext) {
        self.after.fetch_add(1, Ordering::SeqCst);
    }
    async fn on_error(&self, _ctx: &CmpContext, _e: &LiteflowError) {
        self.errors.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn global_aspect() {
    let bus = FlowBus::new();
    let (b, a, e) = (
        Arc::new(AtomicUsize::new(0)),
        Arc::new(AtomicUsize::new(0)),
        Arc::new(AtomicUsize::new(0)),
    );
    bus.register_aspect(Arc::new(CountAspect {
        before: b.clone(),
        after: a.clone(),
        errors: e.clone(),
    }));
    bus.register("ok", cmp(|_| async { null_ok() }));
    bus.register(
        "bad",
        cmp(|_| async { Err(LiteflowError::Custom("x".into())) }),
    );
    bus.add_chain("c1", "THEN(ok, ok, bad)").unwrap();
    let resp = bus.execute("c1").await;
    assert!(!resp.is_success());
    assert_eq!(b.load(Ordering::SeqCst), 3);
    assert_eq!(a.load(Ordering::SeqCst), 3);
    assert_eq!(e.load(Ordering::SeqCst), 1);
}

// ---------- MonitorBus 统计 ----------

#[tokio::test]
async fn monitor_bus_statistics() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { null_ok() }));
    bus.add_chain("c1", "THEN(a, a, a)").unwrap();
    for _ in 0..3 {
        bus.execute("c1").await;
    }
    let report = bus.monitor().report();
    let stat = report.iter().find(|s| s.node_id == "a").unwrap();
    assert_eq!(stat.total, 9);
    assert_eq!(stat.success, 9);
    assert_eq!(stat.fail, 0);
    assert_eq!(bus.monitor().statistics_map()["a"].len(), 9);
    assert!(
        bus.monitor()
            .print_statistics()
            .contains("COMPONENT[a] AVERAGE TIME SPENT :")
    );
}

#[tokio::test]
async fn monitor_time_task_runs_on_real_tokio_schedule() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { null_ok() }));
    bus.add_chain("monitorTaskChain", "THEN(a)").unwrap();
    assert!(bus.execute("monitorTaskChain").await.is_success());

    let runs = Arc::new(AtomicUsize::new(0));
    let runs_for_sink = runs.clone();
    let task = Arc::new(MonitorTimeTask::with_sink(
        bus.monitor().clone(),
        move |report| {
            assert!(report.contains("COMPONENT[a]"));
            runs_for_sink.fetch_add(1, Ordering::SeqCst);
        },
    ));
    let handle = task.spawn(Duration::from_millis(1), Duration::from_millis(5));

    tokio::time::timeout(Duration::from_secs(1), async {
        while runs.load(Ordering::SeqCst) < 2 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    handle.abort();
    assert!(runs.load(Ordering::SeqCst) >= 2);
}

// ---------- 生命周期钩子 ----------

struct HookLog {
    events: Arc<std::sync::Mutex<Vec<String>>>,
}

impl liteflow_core::LifeCycle for HookLog {
    // 与 Java addLifeCycle 的 else-if 顺序一致：同时实现多个阶段时，
    // ChainBuild 分支先于 FlowExecute 分支登记。
    fn register_life_cycle(
        self: Arc<Self>,
        life_cycle_holder: &mut liteflow_core::LifeCycleHolder,
    ) {
        life_cycle_holder.chain_build.push(self);
    }
}

impl liteflow_core::lifecycle::PostProcessChainBuildLifeCycle for HookLog {
    fn post_process_after_chain_build(&self, chain_id: &str) {
        self.events
            .lock()
            .unwrap()
            .push(format!("chain_build:{chain_id}"));
    }
}

#[async_trait::async_trait]
impl liteflow_core::lifecycle::PostProcessFlowExecuteLifeCycle for HookLog {
    async fn post_process_before_flow_execute(&self, chain_id: &str) {
        self.events
            .lock()
            .unwrap()
            .push(format!("before_exec:{chain_id}"));
    }
    async fn post_process_after_flow_execute(&self, chain_id: &str) {
        self.events
            .lock()
            .unwrap()
            .push(format!("after_exec:{chain_id}"));
    }
}

#[tokio::test]
async fn lifecycle_hooks() {
    let bus = FlowBus::new();
    let events = Arc::new(std::sync::Mutex::new(Vec::new()));
    let hook = Arc::new(HookLog {
        events: events.clone(),
    });
    bus.register_chain_build_hook(hook.clone());
    bus.register_flow_execute_hook(hook);
    bus.register("a", cmp(|_| async { null_ok() }));
    bus.add_chain("c1", "THEN(a)").unwrap();
    bus.execute("c1").await;
    let log = events.lock().unwrap().clone();
    assert!(log.contains(&"chain_build:c1".to_string()));
    assert!(log.contains(&"before_exec:c1".to_string()));
    assert!(log.contains(&"after_exec:c1".to_string()));
}

// ---------- NodeInstanceId ----------

#[tokio::test]
async fn node_instance_id() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { null_ok() }));
    bus.add_chain("c1", "THEN(a, a, a)").unwrap();
    let resp = bus.execute("c1").await;
    assert_eq!(resp.steps.len(), 3);
}

#[tokio::test]
async fn xml_inheritance() {
    let bus = FlowBus::new();
    bus.register(
        "a",
        cmp(|ctx| async move {
            ctx.set_data("r", json!("a"));
            Ok(Value::Null)
        }),
    );
    bus.register(
        "b",
        cmp(|ctx| async move {
            ctx.set_data("r", json!("b"));
            Ok(Value::Null)
        }),
    );
    let xml = r#"<flow>
        <chain name="parent">THEN(a, {{x}})</chain>
        <chain name="child" extends="parent">{{x}} = b;</chain>
    </flow>"#;
    let ids = rule::load_xml_str(&bus, xml).unwrap();
    assert_eq!(ids, vec!["child"]);
    assert_eq!(bus.execute("child").await.data("r"), Some(json!("b")));
}
