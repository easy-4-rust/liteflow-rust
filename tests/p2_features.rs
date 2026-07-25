//! P2 迁移项测试：YML 规则 / 链继承 / 子链嵌套 / 声明式组件 / AOP / 监控 / 生命周期 / 实例编号。

use liteflow_rust::{cmp, rule, CmpContext, FlowBus, LiteflowError};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn null_ok() -> Result<Value, LiteflowError> {
    Ok(Value::Null)
}

// ---------- YML 规则 ----------

#[tokio::test]
async fn yml_rule_loading() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|ctx| async move {
        ctx.set_data("r", json!("ran"));
        Ok(Value::Null)
    }));
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
    bus.register("a", cmp(|ctx| async move {
        ctx.set_data("hit", json!(true));
        Ok(Value::Null)
    }));
    let yml = r#"
flow:
  chain:
    - id: rc
      namespace: ns1
      route: r1
      body: THEN(a)
"#;
    rule::load_yml_str(&bus, yml).unwrap();
    let resps = bus.execute_route_chain(Some("ns1"), Value::Null).await.unwrap();
    assert_eq!(resps.len(), 1);
}

// ---------- 链继承（extends + {{占位符}}） ----------

#[tokio::test]
async fn chain_inheritance() {
    let bus = FlowBus::new();
    for id in ["a", "b", "c", "d"] {
        bus.register(id, cmp(|ctx| async move {
            let mut v: Vec<String> = ctx.get_data_as("seq").unwrap_or_default();
            v.push(ctx.node_id().to_string());
            ctx.set_data("seq", json!(v));
            Ok(Value::Null)
        }));
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
        bus.register(id, cmp(|ctx| async move {
            let mut seq: Vec<String> = ctx.get_data_as("seq").unwrap_or_default();
            seq.push(ctx.node_id().to_string());
            ctx.set_data("seq", json!(seq));
            Ok(Value::Null)
        }));
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
impl liteflow_rust::core::decl_component::DeclComponent for OrderCmp {
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
    bus.register("vip", cmp(|ctx| async move {
        ctx.set_data("plan", json!("vip"));
        Ok(Value::Null)
    }));
    bus.register("normal", cmp(|ctx| async move {
        ctx.set_data("plan", json!("normal"));
        Ok(Value::Null)
    }));
    bus.add_chain("c1", "THEN(orderCmp.checkStock, IF(orderCmp.isVip, vip, normal))")
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
impl liteflow_rust::aop::CmpAroundAspect for CountAspect {
    async fn before(&self, _ctx: &CmpContext) {
        self.before.fetch_add(1, Ordering::SeqCst);
    }
    async fn after(&self, _ctx: &CmpContext) {
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
    bus.register("bad", cmp(|_| async { Err(LiteflowError::Custom("x".into())) }));
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
}

// ---------- 生命周期钩子 ----------

struct HookLog {
    events: Arc<std::sync::Mutex<Vec<String>>>,
}

impl liteflow_rust::lifecycle::PostProcessChainBuildLifeCycle for HookLog {
    fn post_process_after_chain_build(&self, chain_id: &str) {
        self.events.lock().unwrap().push(format!("chain_build:{chain_id}"));
    }
}

#[async_trait::async_trait]
impl liteflow_rust::lifecycle::PostProcessFlowExecuteLifeCycle for HookLog {
    async fn post_process_before_flow_execute(&self, chain_id: &str) {
        self.events.lock().unwrap().push(format!("before_exec:{chain_id}"));
    }
    async fn post_process_after_flow_execute(&self, chain_id: &str) {
        self.events.lock().unwrap().push(format!("after_exec:{chain_id}"));
    }
}

#[tokio::test]
async fn lifecycle_hooks() {
    let bus = FlowBus::new();
    let events = Arc::new(std::sync::Mutex::new(Vec::new()));
    let hook = Arc::new(HookLog { events: events.clone() });
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
    bus.register("a", cmp(|ctx| async move {
        ctx.set_data("r", json!("a"));
        Ok(Value::Null)
    }));
    bus.register("b", cmp(|ctx| async move {
        ctx.set_data("r", json!("b"));
        Ok(Value::Null)
    }));
    let xml = r#"<flow>
        <chain name="parent">THEN(a, {{x}})</chain>
        <chain name="child" extends="parent">{{x}} = b;</chain>
    </flow>"#;
    let ids = rule::load_xml_str(&bus, xml).unwrap();
    assert_eq!(ids, vec!["child"]);
    assert_eq!(bus.execute("child").await.data("r"), Some(json!("b")));
}
