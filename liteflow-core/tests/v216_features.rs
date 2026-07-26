//! v2.16.0 新增语义测试：
//! execute2RespWithEL / ExecuteOption / FlowEvent / Slot attachment /
//! Condition 级 bind / NodeId 校验 / AND-OR isAccess 过滤 / execute2RespWithRid

use liteflow_core::{
    ExecuteOption, FlowBus, FlowEvent, LiteflowError, NodeComponent, cmp, listener,
};
use md5::{Digest, Md5};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

// ---------- execute2RespWithEL（2.16） ----------

#[tokio::test]
async fn execute_with_el_direct() {
    let bus = FlowBus::new();
    bus.register(
        "a",
        cmp(|ctx| async move {
            ctx.set_data("ran", json!(true));
            Ok(Value::Null)
        }),
    );
    bus.register("b", cmp(|_| async move { Ok(Value::Null) }));

    // 直接执行 EL 字符串（无需先 add_chain）
    let resp = bus.execute_with_el("THEN(a, b)").await;
    assert!(resp.is_success());

    // elMd5 缓存：同一 EL 复用同一匿名链（空白/单引号差异也被 normalize）
    let resp2 = bus.execute_with_el(" THEN( a , b ) ; ").await;
    assert!(resp2.is_success());
    assert_eq!(
        bus.chain_ids().len(),
        1,
        "相同 EL 应复用 elMd5 缓存的匿名链"
    );

    let resp3 = bus.execute_with_el_data("THEN(a)", json!({"k": 1})).await;
    assert!(resp3.is_success());
}

#[tokio::test]
async fn execute_with_el_invalid() {
    let bus = FlowBus::new();
    let resp = bus.execute_with_el("THEN(a, b)").await; // 节点未注册
    assert!(!resp.is_success());
}

// ---------- ExecuteOption / requestId / conversationId（2.16） ----------

#[tokio::test]
async fn execute_with_option_rid_cid() {
    let bus = FlowBus::new();
    bus.register(
        "a",
        cmp(|ctx| async move {
            assert_eq!(ctx.request_id(), "RID-001");
            assert_eq!(ctx.conversation_id(), Some("CID-001"));
            Ok(Value::Null)
        }),
    );
    bus.add_chain("c1", "THEN(a)").unwrap();

    let opt = ExecuteOption::of()
        .request_id("RID-001")
        .conversation_id("CID-001");
    let resp = bus.execute_with_option("c1", Value::Null, opt).await;
    assert!(resp.is_success());
    assert_eq!(resp.request_id, "RID-001");
}

#[tokio::test]
async fn execute_with_auto_conversation_id() {
    let bus = FlowBus::new();
    bus.register(
        "a",
        cmp(|ctx| async move {
            assert!(ctx.conversation_id().is_some());
            Ok(Value::Null)
        }),
    );
    bus.add_chain("c1", "THEN(a)").unwrap();
    let opt = ExecuteOption::of().auto_conversation_id();
    let resp = bus.execute_with_option("c1", Value::Null, opt).await;
    assert!(resp.is_success());
}

// ---------- execute2RespWithRid（组件内同一 requestId 调子链） ----------

#[tokio::test]
async fn execute_with_rid_keeps_request_id() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async move { Ok(Value::Null) }));
    bus.add_chain("sub", "THEN(a)").unwrap();
    let resp = bus.execute_with_rid("sub", Value::Null, "RID-KEEP").await;
    assert!(resp.is_success());
    assert_eq!(resp.request_id, "RID-KEEP");
}

// ---------- FlowEvent / FlowEventListener / FlowEventPublisher（2.15+） ----------

#[tokio::test]
async fn flow_event_publish_to_listener() {
    let bus = FlowBus::new();
    bus.register(
        "a",
        cmp(|ctx| async move {
            ctx.publish_event(
                &FlowEvent::builder("node.text")
                    .node_id("a")
                    .chain_id(ctx.chain_id())
                    .request_id(ctx.request_id())
                    .text("hello")
                    .last(true)
                    .build(),
            );
            Ok(Value::Null)
        }),
    );
    bus.add_chain("c1", "THEN(a)").unwrap();

    let events = Arc::new(Mutex::new(Vec::new()));
    let events2 = events.clone();
    let opt = ExecuteOption::of().event_listener(Arc::new(listener(move |e| {
        events2
            .lock()
            .unwrap()
            .push((e.event_type.clone(), e.text.clone(), e.last));
    })));
    let resp = bus.execute_with_option("c1", Value::Null, opt).await;
    assert!(resp.is_success());
    let evs = events.lock().unwrap();
    assert_eq!(evs.len(), 1);
    assert_eq!(evs[0].0, "node.text");
    assert_eq!(evs[0].1.as_deref(), Some("hello"));
    assert!(evs[0].2);
}

#[tokio::test]
async fn flow_event_no_listener_noop() {
    let bus = FlowBus::new();
    bus.register(
        "a",
        cmp(|ctx| async move {
            // 无 listener 时静默忽略（对齐 Java publish 语义）
            ctx.publish_event(&FlowEvent::builder("x").build());
            Ok(Value::Null)
        }),
    );
    bus.add_chain("c1", "THEN(a)").unwrap();
    assert!(bus.execute("c1").await.is_success());
}

// ---------- Slot attachment（2.15+） ----------

#[tokio::test]
async fn slot_attachment_crud() {
    let bus = FlowBus::new();
    bus.register(
        "a",
        cmp(|ctx| async move {
            ctx.set_attachment("k1", 42i32);
            assert!(ctx.has_attachment("k1"));
            let v: Option<Arc<i32>> = ctx.get_attachment("k1");
            assert_eq!(*v.unwrap(), 42);
            ctx.remove_attachment("k1");
            assert!(!ctx.has_attachment("k1"));
            Ok(Value::Null)
        }),
    );
    bus.add_chain("c1", "THEN(a)").unwrap();
    assert!(bus.execute("c1").await.is_success());
}

// ---------- Condition 级 bind（2.14+） ----------

#[tokio::test]
async fn condition_level_bind() {
    let bus = FlowBus::new();
    bus.register(
        "a",
        cmp(|ctx| async move {
            ctx.set_data("bind_v", json!(ctx.bind_data("mk")));
            Ok(Value::Null)
        }),
    );
    // THEN(...).bind("mk", "mv")：Condition 级 bind 对子节点可见
    bus.add_chain("c1", "THEN(a).bind(\"mk\", \"mv\")").unwrap();
    let resp = bus.execute("c1").await;
    assert!(resp.is_success(), "{}", resp.message);
    assert_eq!(resp.data("bind_v").unwrap(), json!("mv"));
}

#[tokio::test]
async fn condition_bind_override_clears_node_bind() {
    let bus = FlowBus::new();
    bus.register(
        "a",
        cmp(|ctx| async move {
            ctx.set_data("bind_v", json!(ctx.bind_data("mk")));
            Ok(Value::Null)
        }),
    );
    // 节点级 bind 为 "node_v"，Condition 级 bind override=true 应覆盖为 "cond_v"
    bus.add_chain(
        "c1",
        "THEN(a.bind(\"mk\", \"node_v\")).bind(\"mk\", \"cond_v\", true)",
    )
    .unwrap();
    let resp = bus.execute("c1").await;
    assert!(resp.is_success(), "{}", resp.message);
    assert_eq!(resp.data("bind_v").unwrap(), json!("cond_v"));
}

#[tokio::test]
async fn chain_bind_via_wrapper() {
    let bus = FlowBus::new();
    bus.register(
        "a",
        cmp(|ctx| async move {
            ctx.set_data("bind_v", json!(ctx.bind_data("mk")));
            Ok(Value::Null)
        }),
    );
    bus.register("b", cmp(|_| async move { Ok(Value::Null) }));
    bus.add_chain("sub", "THEN(a)").unwrap();
    // 子链 id.bind(...)：bind 数据存在 ChainBindWrapperCondition 上，子链内可见
    bus.add_chain("main", "THEN(sub.bind(\"mk\", \"sub_v\"), b)")
        .unwrap();
    let resp = bus.execute("main").await;
    assert!(resp.is_success(), "{}", resp.message);
    assert_eq!(resp.data("bind_v").unwrap(), json!("sub_v"));
}

// ---------- NodeId 合法性校验（2.16） ----------

#[test]
fn node_id_validation() {
    let bus = FlowBus::new();
    assert!(
        bus.try_register("ok_id$1", cmp(|_| async move { Ok(Value::Null) }))
            .is_ok()
    );
    let err = bus.try_register("1bad", cmp(|_| async move { Ok(Value::Null) }));
    assert!(matches!(err, Err(LiteflowError::NodeIdUnIllegal(_))));
    let err2 = bus.try_register("bad-id", cmp(|_| async move { Ok(Value::Null) }));
    assert!(matches!(err2, Err(LiteflowError::NodeIdUnIllegal(_))));
    let err3 = bus.try_register("", cmp(|_| async move { Ok(Value::Null) }));
    assert!(matches!(err3, Err(LiteflowError::NodeIdUnIllegal(_))));
}

// ---------- AND/OR 的 isAccess 过滤（2.16） ----------

struct InaccessibleBool;
#[async_trait::async_trait]
impl NodeComponent for InaccessibleBool {
    async fn process(&self, _ctx: &liteflow_core::CmpContext) -> Result<Value, LiteflowError> {
        panic!("不可访问的节点不应被执行");
    }
    fn is_access(&self, _ctx: &liteflow_core::CmpContext) -> bool {
        false
    }
}

#[tokio::test]
async fn and_or_filters_inaccessible_items() {
    let bus = FlowBus::new();
    bus.register("skip", InaccessibleBool);
    bus.register("t", cmp(|_| async move { Ok(json!(true)) }));

    // AND：不可访问项被排除，不参与求值（旧语义会因返回 Null 报类型错误）
    bus.add_chain("c_and", "IF(AND(skip, t), t)").unwrap();
    let resp = bus.execute("c_and").await;
    assert!(resp.is_success(), "{}", resp.message);

    // OR：不可访问项被排除
    bus.add_chain("c_or", "IF(OR(skip, t), t)").unwrap();
    let resp = bus.execute("c_or").await;
    assert!(resp.is_success(), "{}", resp.message);

    // 全部不可访问：AND 为空集 → true（allMatch 空集语义）
    bus.add_chain("c_all_skip", "IF(AND(skip), t)").unwrap();
    let resp = bus.execute("c_all_skip").await;
    assert!(resp.is_success(), "{}", resp.message);
}

// ---------- ScriptValidator.validateWithEx（2.16） ----------

#[test]
fn script_validate_ex() {
    let exec = liteflow_core::script::script_executor::RhaiScriptExecutor::new();
    assert!(exec.validate("1 + 1"));
    assert!(exec.validate_ex("1 + 1").is_ok());
    assert!(!exec.validate("let x = "));
    assert!(exec.validate_ex("let x = ").is_err());
}

// ---------- elMd5 索引随 remove_chain 清理（2.16） ----------

#[tokio::test]
async fn el_md5_index_cleanup() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async move { Ok(Value::Null) }));
    bus.execute_with_el("THEN(a)").await;
    let md5 = format!(
        "{:x}",
        Md5::digest(liteflow_core::util::el_regex_util::normalize_el("THEN(a)").as_bytes())
    );
    let chain_id = bus.get_chain_id_by_el_md5(&md5).unwrap();
    bus.remove_chain(&chain_id);
    assert!(bus.get_chain_id_by_el_md5(&md5).is_none());
}
