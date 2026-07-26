//! P1 功能测试：rhai 脚本节点 / XML 规则 / route 决策表链路。

use liteflow_core::{FlowBus, LiteflowError, ScriptKind, cmp, rule};
use serde_json::{Value, json};

// ---------- rhai 脚本节点 ----------

#[tokio::test]
async fn script_common_node() {
    let bus = FlowBus::new();
    bus.register_script(
        "s1",
        "rhai",
        r#"
        let amount = input.amount;
        data.total = amount * 2;
        data.total
    "#,
    )
    .unwrap();
    bus.add_chain("c1", "THEN(s1)").unwrap();
    let resp = bus.execute_with_data("c1", json!({"amount": 21})).await;
    assert!(resp.is_success());
    assert_eq!(resp.data("total"), Some(json!(42)));
}

#[tokio::test]
async fn script_boolean_in_if() {
    let bus = FlowBus::new();
    bus.register_script_typed("check", "rhai", ScriptKind::Boolean, "input.score >= 60")
        .unwrap();
    bus.register(
        "pass",
        cmp(|ctx| async move {
            ctx.set_data("result", json!("pass"));
            Ok(Value::Null)
        }),
    );
    bus.register(
        "fail",
        cmp(|ctx| async move {
            ctx.set_data("result", json!("fail"));
            Ok(Value::Null)
        }),
    );
    bus.add_chain("c1", "IF(check, pass, fail)").unwrap();

    let resp = bus.execute_with_data("c1", json!({"score": 80})).await;
    assert_eq!(resp.data("result"), Some(json!("pass")));
    let resp = bus.execute_with_data("c1", json!({"score": 30})).await;
    assert_eq!(resp.data("result"), Some(json!("fail")));
}

#[tokio::test]
async fn script_switch_and_for() {
    let bus = FlowBus::new();
    bus.register_script_typed("sw", "rhai", ScriptKind::Switch, r#"input.route"#)
        .unwrap();
    bus.register_script_typed("counter", "rhai", ScriptKind::For, "3")
        .unwrap();
    let n = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let n2 = n.clone();
    bus.register(
        "a",
        cmp(|ctx| async move {
            ctx.set_data("hit", json!("a"));
            Ok(Value::Null)
        }),
    );
    bus.register(
        "b",
        cmp(move |ctx| {
            let n = n2.clone();
            async move {
                n.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                ctx.set_data("hit", json!("b"));
                Ok(Value::Null)
            }
        }),
    );
    bus.add_chain("c1", "THEN(SWITCH(sw).TO(a, b), FOR(counter).DO(b))")
        .unwrap();
    let resp = bus.execute_with_data("c1", json!({"route": "a"})).await;
    assert!(resp.is_success());
    assert_eq!(resp.data("hit"), Some(json!("b"))); // FOR 覆盖了 SWITCH 的 hit
    assert_eq!(n.load(std::sync::atomic::Ordering::SeqCst), 3);
}

#[tokio::test]
async fn script_type_error_checked() {
    let bus = FlowBus::new();
    // boolean_script 返回字符串 → NodeTypeError
    bus.register_script_typed("bad", "rhai", ScriptKind::Boolean, r#""not a bool""#)
        .unwrap();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    bus.add_chain("c1", "IF(bad, a)").unwrap();
    let resp = bus.execute("c1").await;
    assert!(!resp.is_success());
}

#[tokio::test]
async fn script_unsupported_language() {
    let bus = FlowBus::new();
    let err = bus.register_script("g1", "groovy", "1+1").unwrap_err();
    assert!(matches!(err, LiteflowError::Script { .. }));
}

#[tokio::test]
async fn script_loop_context() {
    let bus = FlowBus::new();
    bus.register_script_typed("it", "rhai", ScriptKind::Iterator, "[10, 20, 30]")
        .unwrap();
    bus.register_script(
        "acc",
        "rhai",
        r#"
        if data.contains("sum") { data.sum += loop_object; } else { data.sum = loop_object; }
    "#,
    )
    .unwrap();
    bus.add_chain("c1", "ITERATOR(it).DO(acc)").unwrap();
    let resp = bus.execute("c1").await;
    assert!(resp.is_success());
    assert_eq!(resp.data("sum"), Some(json!(60)));
}

// ---------- XML 规则 ----------

#[tokio::test]
async fn xml_rule_basic() {
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
        <chain name="c1">THEN(a, b)</chain>
        <chain name="c2" enable="false">THEN(b)</chain>
    </flow>"#;
    let ids = rule::load_xml_str(&bus, xml).unwrap();
    assert_eq!(ids, vec!["c1"]);
    assert!(!bus.contains_chain("c2")); // enable=false 被跳过
    let resp = bus.execute("c1").await;
    assert!(resp.is_success());
    assert_eq!(resp.data("r"), Some(json!("b")));
}

#[tokio::test]
async fn xml_rule_with_script_nodes() {
    let bus = FlowBus::new();
    bus.register(
        "hit",
        cmp(|ctx| async move {
            ctx.set_data("hit", json!(true));
            Ok(Value::Null)
        }),
    );
    let xml = r#"<flow>
        <nodes>
            <node id="check" type="boolean_script" language="rhai"><![CDATA[input.v > 5]]></node>
        </nodes>
        <chain name="c1">IF(check, hit)</chain>
    </flow>"#;
    rule::load_xml_str(&bus, xml).unwrap();
    let resp = bus.execute_with_data("c1", json!({"v": 10})).await;
    assert_eq!(resp.data("hit"), Some(json!(true)));
    let resp = bus.execute_with_data("c1", json!({"v": 1})).await;
    assert_eq!(resp.data("hit"), None);
}

#[tokio::test]
async fn xml_route_requires_body() {
    let bus = FlowBus::new();
    let xml = r#"<flow>
        <chain name="c1"><route>IF(x)</route></chain>
    </flow>"#;
    let err = rule::load_xml_str(&bus, xml).unwrap_err();
    assert!(err.to_string().contains("body"));
}

// ---------- route 决策表链路 ----------

#[tokio::test]
async fn route_chain_matched() {
    let bus = FlowBus::new();
    bus.register(
        "r1",
        cmp(|ctx| async move {
            let v: i64 = ctx
                .request_data::<serde_json::Value>()
                .and_then(|v| v.get("level").and_then(|x| x.as_i64()))
                .unwrap_or(0);
            Ok(json!(v >= 5))
        }),
    );
    bus.register(
        "r2",
        cmp(|ctx| async move {
            let v: i64 = ctx
                .request_data::<serde_json::Value>()
                .and_then(|v| v.get("level").and_then(|x| x.as_i64()))
                .unwrap_or(0);
            Ok(json!(v < 5))
        }),
    );
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

    // 决策表链路：route 命中才执行 body
    bus.add_route_chain("vipChain", "order", "r1", "THEN(vip)")
        .unwrap();
    bus.add_route_chain("normalChain", "order", "r2", "THEN(normal)")
        .unwrap();

    let resps = bus
        .execute_route_chain(Some("order"), json!({"level": 8}))
        .await
        .unwrap();
    assert_eq!(resps.len(), 1);
    assert_eq!(resps[0].chain_id, "vipChain");
    assert_eq!(resps[0].data("plan"), Some(json!("vip")));

    let resps = bus
        .execute_route_chain(Some("order"), json!({"level": 1}))
        .await
        .unwrap();
    assert_eq!(resps.len(), 1);
    assert_eq!(resps[0].chain_id, "normalChain");
}

#[tokio::test]
async fn route_chain_no_match_error() {
    let bus = FlowBus::new();
    bus.register("r", cmp(|_| async { Ok(json!(false)) }));
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    bus.add_route_chain("c1", "ns", "r", "THEN(a)").unwrap();
    let err = bus
        .execute_route_chain(Some("ns"), Value::Null)
        .await
        .unwrap_err();
    assert!(matches!(err, LiteflowError::NoMatchedRouteChain));
    // namespace 无 route 链路
    let err = bus
        .execute_route_chain(Some("other"), Value::Null)
        .await
        .unwrap_err();
    assert!(matches!(err, LiteflowError::RouteChainNotFound(_)));
}

#[tokio::test]
async fn route_via_xml_and_json() {
    let bus = FlowBus::new();
    bus.register(
        "r1",
        cmp(|ctx| async move {
            let flag: bool = ctx
                .request_data::<serde_json::Value>()
                .and_then(|v| v.get("flag").and_then(|x| x.as_bool()))
                .unwrap_or(false);
            Ok(json!(flag))
        }),
    );
    bus.register(
        "a",
        cmp(|ctx| async move {
            ctx.set_data("hit", json!("A"));
            Ok(Value::Null)
        }),
    );

    let xml = r#"<flow>
        <chain name="cA" namespace="ns1">
            <route>r1</route>
            <body>THEN(a)</body>
        </chain>
    </flow>"#;
    rule::load_xml_str(&bus, xml).unwrap();
    let resps = bus
        .execute_route_chain(Some("ns1"), json!({"flag": true}))
        .await
        .unwrap();
    assert_eq!(resps.len(), 1);
    assert_eq!(resps[0].data("hit"), Some(json!("A")));

    let json_rule = r#"{"flow":{"chain":[
        {"id":"cB","namespace":"ns2","route":"r1","body":"THEN(a)"}
    ]}}"#;
    rule::load_json_str(&bus, json_rule).unwrap();
    let resps = bus
        .execute_route_chain(Some("ns2"), json!({"flag": true}))
        .await
        .unwrap();
    assert_eq!(resps.len(), 1);
    assert_eq!(resps[0].chain_id, "cB");
}
