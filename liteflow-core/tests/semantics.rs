//! 执行语义集成测试，对齐 LiteFlow Java 版测试用例的核心行为。

use liteflow_core::{cmp, FlowBus, LiteflowError};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn then_sequential() {
    let bus = FlowBus::new();
    let log = Arc::new(std::sync::Mutex::new(Vec::<&str>::new()));
    for id in ["a", "b", "c"] {
        let log = log.clone();
        bus.register(id, cmp(move |_| {
            let log = log.clone();
            async move {
                log.lock().unwrap().push(id);
                Ok(Value::Null)
            }
        }));
    }
    bus.add_chain("c1", "THEN(a, b, c)").unwrap();
    let resp = bus.execute("c1").await;
    assert!(resp.is_success());
    assert_eq!(*log.lock().unwrap(), vec!["a", "b", "c"]);
    assert_eq!(resp.steps.len(), 3);
}

#[tokio::test]
async fn when_parallel_all() {
    let bus = FlowBus::new();
    let counter = Arc::new(AtomicUsize::new(0));
    for id in ["a", "b", "c"] {
        let counter = counter.clone();
        bus.register(id, cmp(move |_| {
            let counter = counter.clone();
            async move {
                tokio::time::sleep(Duration::from_millis(50)).await;
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(Value::Null)
            }
        }));
    }
    bus.add_chain("c1", "WHEN(a, b, c)").unwrap();
    let start = std::time::Instant::now();
    let resp = bus.execute("c1").await;
    assert!(resp.is_success());
    assert_eq!(counter.load(Ordering::SeqCst), 3);
    assert!(start.elapsed() < Duration::from_millis(140));
}

#[tokio::test]
async fn when_error_propagates_unless_ignore() {
    let bus = FlowBus::new();
    bus.register("ok", cmp(|_| async { Ok(Value::Null) }));
    bus.register("bad", cmp(|_| async { Err(LiteflowError::Custom("boom".into())) }));
    bus.add_chain("c1", "WHEN(ok, bad)").unwrap();
    bus.add_chain("c2", "WHEN(ok, bad).ignore_error(true)").unwrap();
    assert!(!bus.execute("c1").await.is_success());
    assert!(bus.execute("c2").await.is_success());
}

#[tokio::test]
async fn when_any_returns_on_first() {
    let bus = FlowBus::new();
    bus.register("slow", cmp(|_| async {
        tokio::time::sleep(Duration::from_millis(500)).await;
        Ok(Value::Null)
    }));
    bus.register("fast", cmp(|_| async { Ok(Value::Null) }));
    bus.add_chain("c1", "WHEN(slow, fast).ANY(true)").unwrap();
    let start = std::time::Instant::now();
    let resp = bus.execute("c1").await;
    assert!(resp.is_success());
    assert!(start.elapsed() < Duration::from_millis(200));
}

#[tokio::test]
async fn when_timeout() {
    let bus = FlowBus::new();
    bus.register("slow", cmp(|_| async {
        tokio::time::sleep(Duration::from_millis(300)).await;
        Ok(Value::Null)
    }));
    bus.add_chain("c1", "WHEN(slow).MAX_WAIT_MILLISECONDS(80)").unwrap();
    let resp = bus.execute("c1").await;
    assert!(!resp.is_success());
    assert!(resp.message.contains("timeout"));
}

#[tokio::test]
async fn if_else_branch() {
    let bus = FlowBus::new();
    bus.register("check", cmp(|ctx| async move {
        let flag: bool = ctx.get_data_as("flag").unwrap_or(false);
        Ok(Value::Bool(flag))
    }));
    bus.register("yes", cmp(|ctx| async move {
        ctx.set_data("route", json!("yes"));
        Ok(Value::Null)
    }));
    bus.register("no", cmp(|ctx| async move {
        ctx.set_data("route", json!("no"));
        Ok(Value::Null)
    }));
    bus.add_chain("c1", "IF(check, yes, no)").unwrap();
    let resp = bus.execute("c1").await;
    assert!(resp.is_success());
    assert_eq!(resp.data("route"), Some(json!("no")));
}

#[tokio::test]
async fn switch_with_tag_and_default() {
    let bus = FlowBus::new();
    bus.register("s", cmp(|_| async { Ok(json!("b:vip")) }));
    bus.register("a", cmp(|ctx| async move {
        ctx.set_data("hit", json!("a"));
        Ok(Value::Null)
    }));
    bus.register("b", cmp(|ctx| async move {
        let tag = ctx.tag().unwrap_or("").to_string();
        ctx.set_data("hit", json!(format!("b:{tag}")));
        Ok(Value::Null)
    }));
    bus.register("d", cmp(|ctx| async move {
        ctx.set_data("hit", json!("default"));
        Ok(Value::Null)
    }));
    bus.add_chain("c1", r#"SWITCH(s).TO(a, "b:vip").DEFAULT(d)"#).unwrap();
    let resp = bus.execute("c1").await;
    assert!(resp.is_success());
    assert_eq!(resp.data("hit"), Some(json!("b:vip")));

    bus.reload_chain("c1", r#"SWITCH(s).TO(a).DEFAULT(d)"#).unwrap();
    let resp = bus.execute("c1").await;
    assert!(resp.is_success());
    assert_eq!(resp.data("hit"), Some(json!("default")));
}

#[tokio::test]
async fn switch_no_target_error() {
    let bus = FlowBus::new();
    bus.register("s", cmp(|_| async { Ok(json!("zzz")) }));
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    bus.add_chain("c1", "SWITCH(s).TO(a)").unwrap();
    let resp = bus.execute("c1").await;
    assert!(!resp.is_success());
}

#[tokio::test]
async fn for_loop_with_break() {
    let bus = FlowBus::new();
    bus.register("f", cmp(|_| async { Ok(json!(10)) }));
    let sum = Arc::new(AtomicUsize::new(0));
    let sum2 = sum.clone();
    bus.register("work", cmp(move |ctx| {
        let sum = sum2.clone();
        async move {
            sum.fetch_add(1, Ordering::SeqCst);
            ctx.set_data("last", json!(ctx.loop_index().unwrap()));
            Ok(Value::Null)
        }
    }));
    bus.register("brk", cmp(|ctx| async move {
        Ok(json!(ctx.loop_index().unwrap() >= 3))
    }));
    bus.add_chain("c1", "FOR(f).DO(work).BREAK(brk)").unwrap();
    let resp = bus.execute("c1").await;
    assert!(resp.is_success());
    assert_eq!(sum.load(Ordering::SeqCst), 4);
}

#[tokio::test]
async fn while_loop() {
    let bus = FlowBus::new();
    let n = Arc::new(AtomicUsize::new(0));
    let n2 = n.clone();
    bus.register("w", cmp(move |_| {
        let n = n2.clone();
        async move {
            let cur = n.fetch_add(1, Ordering::SeqCst);
            Ok(json!(cur < 3))
        }
    }));
    let body_count = Arc::new(AtomicUsize::new(0));
    let bc = body_count.clone();
    bus.register("work", cmp(move |_| {
        let bc = bc.clone();
        async move {
            bc.fetch_add(1, Ordering::SeqCst);
            Ok(Value::Null)
        }
    }));
    bus.add_chain("c1", "WHILE(w).DO(work)").unwrap();
    let resp = bus.execute("c1").await;
    assert!(resp.is_success());
    assert_eq!(body_count.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn iterator_loop_object() {
    let bus = FlowBus::new();
    bus.register("it", cmp(|_| async { Ok(json!(["x", "y", "z"])) }));
    let collected = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let c2 = collected.clone();
    bus.register("work", cmp(move |ctx| {
        let c = c2.clone();
        async move {
            let obj: String = ctx.loop_object().unwrap();
            c.lock().unwrap().push(obj);
            Ok(Value::Null)
        }
    }));
    bus.add_chain("c1", "ITERATOR(it).DO(work)").unwrap();
    let resp = bus.execute("c1").await;
    assert!(resp.is_success());
    assert_eq!(*collected.lock().unwrap(), vec!["x", "y", "z"]);
}

#[tokio::test]
async fn catch_do_swallows() {
    let bus = FlowBus::new();
    bus.register("bad", cmp(|_| async { Err(LiteflowError::Custom("boom".into())) }));
    bus.register("handle", cmp(|ctx| async move {
        ctx.set_data("caught", json!(true));
        Ok(Value::Null)
    }));
    bus.add_chain("c1", "CATCH(bad).DO(handle)").unwrap();
    bus.add_chain("c2", "CATCH(bad)").unwrap();
    let resp = bus.execute("c1").await;
    assert!(resp.is_success());
    assert_eq!(resp.data("caught"), Some(json!(true)));
    assert!(!bus.execute("c2").await.is_success());
}

#[tokio::test]
async fn retry_until_success() {
    let bus = FlowBus::new();
    let tries = Arc::new(AtomicUsize::new(0));
    let t2 = tries.clone();
    bus.register("flaky", cmp(move |_| {
        let t = t2.clone();
        async move {
            if t.fetch_add(1, Ordering::SeqCst) < 2 {
                Err(LiteflowError::Custom("flaky".into()))
            } else {
                Ok(Value::Null)
            }
        }
    }));
    bus.add_chain("c1", "flaky.retry(3)").unwrap();
    let resp = bus.execute("c1").await;
    assert!(resp.is_success());
    assert_eq!(tries.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn pre_and_finally() {
    let bus = FlowBus::new();
    let log = Arc::new(std::sync::Mutex::new(Vec::<&str>::new()));
    for id in ["p", "a", "z"] {
        let log = log.clone();
        bus.register(id, cmp(move |_| {
            let log = log.clone();
            async move {
                log.lock().unwrap().push(id);
                Ok(Value::Null)
            }
        }));
    }
    bus.add_chain("c1", "THEN(PRE(p), a, FINALLY(z))").unwrap();
    let resp = bus.execute("c1").await;
    assert!(resp.is_success());
    assert_eq!(*log.lock().unwrap(), vec!["p", "a", "z"]);
}

#[tokio::test]
async fn finally_runs_on_error() {
    let bus = FlowBus::new();
    bus.register("bad", cmp(|_| async { Err(LiteflowError::Custom("x".into())) }));
    let ran = Arc::new(AtomicUsize::new(0));
    let r2 = ran.clone();
    bus.register("fin", cmp(move |_| {
        let r = r2.clone();
        async move {
            r.fetch_add(1, Ordering::SeqCst);
            Ok(Value::Null)
        }
    }));
    bus.add_chain("c1", "THEN(bad, FINALLY(fin))").unwrap();
    let resp = bus.execute("c1").await;
    assert!(!resp.is_success());
    assert_eq!(ran.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn and_or_not() {
    let bus = FlowBus::new();
    bus.register("t", cmp(|_| async { Ok(json!(true)) }));
    bus.register("f", cmp(|_| async { Ok(json!(false)) }));
    bus.register("hit", cmp(|ctx| async move {
        ctx.set_data("hit", json!(true));
        Ok(Value::Null)
    }));
    bus.add_chain("c1", "IF(AND(t, NOT(f)), hit)").unwrap();
    bus.add_chain("c2", "IF(OR(f, f), hit)").unwrap();
    assert_eq!(bus.execute("c1").await.data("hit"), Some(json!(true)));
    assert_eq!(bus.execute("c2").await.data("hit"), None);
}

#[tokio::test]
async fn end_chain_semantics() {
    let bus = FlowBus::new();
    bus.register("stopper", cmp(|ctx| async move {
        ctx.end_chain();
        Ok(Value::Null)
    }));
    let ran = Arc::new(AtomicUsize::new(0));
    let r2 = ran.clone();
    bus.register("after", cmp(move |_| {
        let r = r2.clone();
        async move {
            r.fetch_add(1, Ordering::SeqCst);
            Ok(Value::Null)
        }
    }));
    bus.add_chain("c1", "THEN(stopper, after)").unwrap();
    let resp = bus.execute("c1").await;
    assert!(resp.is_success());
    assert_eq!(ran.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn continue_on_error_semantics() {
    struct FlakyCmp;
    #[async_trait::async_trait]
    impl liteflow_core::NodeComponent for FlakyCmp {
        async fn process(&self, _ctx: &liteflow_core::CmpContext) -> Result<Value, LiteflowError> {
            Err(LiteflowError::Custom("ignored".into()))
        }
        fn is_continue_on_error(&self) -> bool {
            true
        }
    }
    let bus = FlowBus::new();
    bus.register("bad", FlakyCmp);
    bus.register("good", cmp(|ctx| async move {
        ctx.set_data("ok", json!(true));
        Ok(Value::Null)
    }));
    bus.add_chain("c1", "THEN(bad, good)").unwrap();
    let resp = bus.execute("c1").await;
    assert!(resp.is_success());
    assert_eq!(resp.data("ok"), Some(json!(true)));
}

#[tokio::test]
async fn chain_reload_smooth() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|ctx| async move {
        ctx.set_data("v", json!("a"));
        Ok(Value::Null)
    }));
    bus.register("b", cmp(|ctx| async move {
        ctx.set_data("v", json!("b"));
        Ok(Value::Null)
    }));
    bus.add_chain("c1", "THEN(a)").unwrap();
    assert_eq!(bus.execute("c1").await.data("v"), Some(json!("a")));
    bus.reload_chain("c1", "THEN(b)").unwrap();
    assert_eq!(bus.execute("c1").await.data("v"), Some(json!("b")));
}

#[tokio::test]
async fn json_rule_loading() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|ctx| async move {
        ctx.set_data("r", json!("ran"));
        Ok(Value::Null)
    }));
    let json_rule = r#"{
        "flow": {
            "chain": [
                {"name": "chainA", "condition": [{"type": "then", "value": "a"}]}
            ]
        }
    }"#;
    let ids = liteflow_core::rule::load_json_str(&bus, json_rule).unwrap();
    assert_eq!(ids, vec!["chainA"]);
    let resp = bus.execute("chainA").await;
    assert!(resp.is_success());
    assert_eq!(resp.data("r"), Some(json!("ran")));
}

#[tokio::test]
async fn chain_timeout_execute() {
    let bus = FlowBus::new();
    bus.register("slow", cmp(|_| async {
        tokio::time::sleep(Duration::from_millis(300)).await;
        Ok(Value::Null)
    }));
    bus.add_chain("c1", "THEN(slow)").unwrap();
    let resp = bus
        .execute_timeout("c1", Value::Null, Duration::from_millis(60))
        .await;
    assert!(!resp.is_success());
}

#[tokio::test]
async fn request_data_and_bean() {
    #[derive(serde::Deserialize)]
    struct Req {
        amount: i64,
    }
    let bus = FlowBus::new();
    bus.register("calc", cmp(|ctx| async move {
        let req: Req = ctx.request_data().unwrap();
        ctx.set_data("doubled", json!(req.amount * 2));
        Ok(Value::Null)
    }));
    bus.add_chain("c1", "THEN(calc)").unwrap();
    let resp = bus.execute_with_data("c1", json!({"amount": 21})).await;
    assert!(resp.is_success());
    assert_eq!(resp.data("doubled"), Some(json!(42)));
}

#[tokio::test]
async fn node_tag_data_alias() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|ctx| async move {
        ctx.set_data("tag", json!(ctx.tag().unwrap_or("")));
        ctx.set_data("data", json!(ctx.cmp_data().unwrap_or("")));
        Ok(Value::Null)
    }));
    bus.add_chain("c1", r#"THEN(a.tag("t1").data("hello").id("a1"))"#).unwrap();
    let resp = bus.execute("c1").await;
    assert!(resp.is_success());
    assert_eq!(resp.data("tag"), Some(json!("t1")));
    assert_eq!(resp.data("data"), Some(json!("hello")));
    assert_eq!(resp.steps[0].node_id, "a1");
}

#[tokio::test]
async fn parallel_for_loop() {
    let bus = FlowBus::new();
    bus.register("f", cmp(|_| async { Ok(json!(4)) }));
    let counter = Arc::new(AtomicUsize::new(0));
    let c2 = counter.clone();
    bus.register("work", cmp(move |_| {
        let c = c2.clone();
        async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            c.fetch_add(1, Ordering::SeqCst);
            Ok(Value::Null)
        }
    }));
    bus.add_chain("c1", "FOR(f).PARALLEL(2).DO(work)").unwrap();
    let start = std::time::Instant::now();
    let resp = bus.execute("c1").await;
    assert!(resp.is_success());
    assert_eq!(counter.load(Ordering::SeqCst), 4);
    assert!(start.elapsed() < Duration::from_millis(150));
}

#[tokio::test]
async fn chain_not_found() {
    let bus = FlowBus::new();
    let resp = bus.execute("nope").await;
    assert!(!resp.is_success());
    assert!(resp.message.contains("not found"));
}
