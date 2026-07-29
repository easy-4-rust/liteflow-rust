//! v2.16.0 新增语义测试：
//! execute2RespWithEL / ExecuteOption / FlowEvent / Slot attachment /
//! Condition 级 bind / NodeId 校验 / AND-OR isAccess 过滤 / execute2RespWithRid

use liteflow_core::{
    ExecuteOption, FlowBus, FlowEvent, FlowExecutor, LiteflowError, NodeComponent, cmp, listener,
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
async fn java_named_flow_executor_methods_preserve_option_and_context_beans() {
    let bus = FlowBus::new();
    bus.register(
        "read_context",
        cmp(|ctx| async move {
            let message = ctx
                .bean::<String>("message")
                .expect("EL 执行应收到 contextBeanArray 对应的具名 Bean");
            ctx.set_data("message_seen", json!(message.as_str()));
            Ok(Value::Null)
        }),
    );
    let executor = FlowExecutor::new(bus);
    let context_beans = vec![(
        "message".to_string(),
        Arc::new("hello".to_string()) as Arc<dyn std::any::Any + Send + Sync>,
    )];

    let response = executor
        .execute2_resp_with_el(
            "THEN(read_context)",
            json!({"source": "java-api"}),
            Some("RID-EL-CONTEXT".to_string()),
            context_beans,
        )
        .await;

    assert!(response.is_success(), "{}", response.message);
    assert_eq!(response.request_id, "RID-EL-CONTEXT");
    assert_eq!(response.data("message_seen"), Some(json!("hello")));

    let option = ExecuteOption::of()
        .request_id("RID-GETTERS")
        .conversation_id("CID-GETTERS")
        .context_bean("message", Arc::new("value".to_string()));
    assert_eq!(option.get_request_id(), Some("RID-GETTERS"));
    assert_eq!(option.get_conversation_id(), Some("CID-GETTERS"));
    assert!(!option.is_auto_conversation_id());
    assert_eq!(option.get_context_beans().len(), 1);
    assert!(option.get_event_listener().is_none());
}

#[tokio::test]
async fn java_named_response_and_future_methods_execute_real_chain() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async move { Ok(Value::Null) }));
    bus.add_chain("java_named", "THEN(a)").unwrap();
    let executor = FlowExecutor::new(bus);

    let response = executor
        .execute2_resp(
            "java_named",
            Value::Null,
            Some(ExecuteOption::of().request_id("RID-RESP")),
        )
        .await;
    assert!(response.is_success(), "{}", response.message);
    assert_eq!(response.request_id, "RID-RESP");

    let future = executor
        .execute2_future_with_rid("java_named", Value::Null, "RID-FUTURE", Vec::new())
        .unwrap();
    let future_response = future.await.unwrap();
    assert!(future_response.is_success(), "{}", future_response.message);
    assert_eq!(future_response.request_id, "RID-FUTURE");
    assert_eq!(
        executor.get_liteflow_config(),
        executor.liteflow_config(),
        "Java 命名 getter 应委托同一配置快照"
    );
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

#[test]
fn flow_event_java_builder_getters_and_serde_are_aligned() {
    let event = FlowEvent::builder()
        .r#type("node.text")
        .chain_id("chain")
        .node_id("node")
        .request_id("request")
        .conversation_id("conversation")
        .text("hello")
        .last(true)
        .data(serde_json::json!({"index": 1}))
        .timestamp(123)
        .build();

    assert_eq!(event.get_type(), "node.text");
    assert_eq!(event.get_chain_id(), Some("chain"));
    assert_eq!(event.get_node_id(), Some("node"));
    assert_eq!(event.get_request_id(), Some("request"));
    assert_eq!(event.get_conversation_id(), Some("conversation"));
    assert_eq!(event.get_text(), Some("hello"));
    assert!(event.is_last());
    assert_eq!(event.get_data(), Some(&serde_json::json!({"index": 1})));
    assert_eq!(event.get_timestamp(), 123);

    let json = serde_json::to_value(&event).expect("FlowEvent 应可序列化");
    assert_eq!(json["type"], "node.text");
    assert_eq!(json["chainId"], "chain");
    assert_eq!(json["conversationId"], "conversation");

    let automatic = FlowEvent::builder().r#type("automatic").build();
    assert!(automatic.get_timestamp() > 0);
}

#[tokio::test]
async fn flow_event_publish_to_listener() {
    let bus = FlowBus::new();
    bus.register(
        "a",
        cmp(|ctx| async move {
            ctx.publish_event(
                &FlowEvent::builder()
                    .r#type("node.text")
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
            ctx.publish_event(&FlowEvent::builder().r#type("x").build());
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
async fn condition_bind_override_is_scoped_to_the_current_key() {
    let bus = FlowBus::new();
    bus.register(
        "a",
        cmp(|ctx| async move {
            ctx.set_data("k1", json!(ctx.bind_data("k1")));
            ctx.set_data("k2", json!(ctx.bind_data("k2")));
            Ok(Value::Null)
        }),
    );
    bus.add_chain(
        "only_k1",
        r#"THEN(a.bind("k1","node1").bind("k2","node2")).bind("k1","cond1",true).bind("k2","cond2",false)"#,
    )
    .unwrap();
    let first = bus.execute("only_k1").await;
    assert!(first.is_success(), "{}", first.message);
    assert_eq!(first.data("k1").unwrap(), json!("cond1"));
    assert_eq!(first.data("k2").unwrap(), json!("node2"));

    bus.add_chain(
        "only_k2",
        r#"THEN(a.bind("k1","node1").bind("k2","node2")).bind("k1","cond1",false).bind("k2","cond2",true)"#,
    )
    .unwrap();
    let second = bus.execute("only_k2").await;
    assert!(second.is_success(), "{}", second.message);
    assert_eq!(second.data("k1").unwrap(), json!("node1"));
    assert_eq!(second.data("k2").unwrap(), json!("cond2"));
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

#[tokio::test]
async fn chain_tag_and_bind_preserve_java_wrapper_type_before_id() {
    let bus = FlowBus::new();
    bus.register(
        "a",
        cmp(|ctx| async move {
            ctx.set_data("bind_v", json!(ctx.bind_data("mk")));
            Ok(Value::Null)
        }),
    );
    bus.add_chain("sub", "THEN(a)").unwrap();

    // Java TagOperator 首先遇到 Chain 时创建 ThenCondition；后续 bind/id
    // 必须继续作用于这个 Condition，而不是把 Chain 当普通 Node 拒绝。
    bus.add_chain(
        "tag_first",
        r#"THEN(sub.tag("tag-first").bind("mk","from-tag-wrapper").id("tag-wrapper"))"#,
    )
    .unwrap();
    let tag_first = bus.execute("tag_first").await;
    assert!(tag_first.is_success(), "{}", tag_first.message);
    assert_eq!(tag_first.data("bind_v").unwrap(), json!("from-tag-wrapper"));

    // Java BindOperator 首先遇到 Chain 时创建 ChainBindWrapperCondition；
    // 后续 tag/id 写入同一包装对象。
    bus.add_chain(
        "bind_first",
        r#"THEN(sub.bind("mk","from-bind-wrapper").tag("bind-first").id("bind-wrapper"))"#,
    )
    .unwrap();
    let bind_first = bus.execute("bind_first").await;
    assert!(bind_first.is_success(), "{}", bind_first.message);
    assert_eq!(
        bind_first.data("bind_v").unwrap(),
        json!("from-bind-wrapper")
    );

    let node_error = bus
        .add_chain("node_id_invalid", r#"THEN(a.tag("node").id("invalid"))"#)
        .expect_err("普通 Node 即使先 tag，仍不能调用 Condition 专属 ID");
    assert!(
        node_error
            .to_string()
            .contains("The caller must be Condition item")
    );
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

    // 参数数量仍按 Java 要求至少为 2；执行期全部不可访问时，
    // AND 的实际求值集合为空，结果为 true（allMatch 空集语义）。
    bus.add_chain("c_all_skip", "IF(AND(skip, skip), t)")
        .unwrap();
    let resp = bus.execute("c_all_skip").await;
    assert!(resp.is_success(), "{}", resp.message);
}

// ---------- ScriptValidator.validateWithEx（2.16） ----------

#[test]
fn script_validate_ex() {
    let exec = liteflow_core::script::RhaiScriptExecutor::new();
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
        Md5::digest(
            liteflow_core::util::el_regex_util::ElRegexUtil::normalize("THEN(a)").as_bytes()
        )
    );
    let chain_id = bus.get_chain_id_by_el_md5(&md5).unwrap();
    assert!(bus.remove_chain(&chain_id));
    assert!(bus.get_chain_id_by_el_md5(&md5).is_none());
    assert!(!bus.remove_chain(&chain_id));
}
