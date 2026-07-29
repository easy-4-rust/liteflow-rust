//! FlowBus 的 Java v2.16 注册、刷新、脚本缓存与执行重载语义验收。

use std::sync::Arc;

use liteflow_core::util::el_regex_util::ElRegexUtil;
use liteflow_core::{ExecuteOption, FlowBus, FlowParserTypeEnum, LiteflowError, NodeTypeEnum, cmp};
use md5::{Digest, Md5};
use serde_json::{Value, json};

/// 验证 reloadChain 不要求 Chain 预先存在，并保留 Java route 重载语义。
///
/// 对应 Java: `FlowBus#reloadChain(String,String,String)`。
#[tokio::test]
async fn reload_chain_creates_missing_chain_and_preserves_or_replaces_route() {
    let bus = FlowBus::default();
    bus.register(
        "body",
        cmp(|ctx| async move {
            ctx.set_data("body", json!(true));
            Ok(Value::Null)
        }),
    );
    bus.add_node(
        "route_true",
        None,
        NodeTypeEnum::Boolean,
        Arc::new(cmp(|_| async { Ok(Value::Bool(true)) })),
    )
    .unwrap();

    bus.reload_chain("created_by_reload", "THEN(body)")
        .expect("Java reloadChain 对不存在 ID 也应创建 Chain");
    assert!(bus.execute("created_by_reload").await.is_success());

    bus.reload_chain_with_route("created_by_reload", "THEN(body)", Some("route_true"))
        .expect("三参数重载应设置 route");
    assert!(
        bus.get_chain_map()["created_by_reload"]
            .get_route_item()
            .is_some()
    );

    // Java setRoute(null) 直接返回；两参数重载不得清除已有 route。
    bus.reload_chain("created_by_reload", "THEN(body)")
        .expect("两参数重载应保留已有 route");
    assert!(
        bus.get_chain_map()["created_by_reload"]
            .get_route_item()
            .is_some()
    );
    let routed = bus
        .execute_route_chain_with_rid(None, Value::Null, "route-request")
        .await
        .expect("保留的 route 应继续参与决策");
    assert_eq!(routed.len(), 1);
    assert_eq!(routed[0].get_request_id(), "route-request");
}

/// 验证 cleanScriptCache 只清编译缓存，reload/unload 的节点注册语义与 Java 一致。
///
/// 对应 Java: `FlowBus#cleanScriptCache`、`#reloadScript` 与
/// `#unloadScriptNode`。
#[tokio::test]
async fn script_cache_clean_reload_and_unload_keep_java_node_map_semantics() {
    let bus = FlowBus::new();
    bus.register("ordinary", cmp(|_| async { Ok(Value::Null) }));

    // Java 对不存在节点和普通节点的 reloadScript 都静默返回。
    bus.reload_script("missing", "40 + 2").unwrap();
    bus.reload_script("ordinary", "40 + 2").unwrap();

    bus.register_script("hot_script", "rhai", r#"data["version"] = 1;"#)
        .unwrap();
    bus.add_chain("hot_chain", "THEN(hot_script)").unwrap();
    assert_eq!(
        bus.execute("hot_chain").await.data("version"),
        Some(json!(1))
    );

    bus.clean_script_cache()
        .expect("单独清理脚本缓存不应删除 nodeMap 元数据");
    assert!(bus.contains_node("hot_script"));

    bus.reload_script("hot_script", r#"data["version"] = 2;"#)
        .expect("保留的脚本元数据应允许重新装载");
    assert_eq!(
        bus.execute("hot_chain").await.data("version"),
        Some(json!(2))
    );

    bus.clean_script_cache().unwrap();
    assert!(
        bus.unload_script_node("hot_script").unwrap(),
        "缓存已经为空时，Java 仍会删除脚本 Node"
    );
    assert!(!bus.contains_node("hot_script"));
}

/// 验证注册表快照、匿名 EL、缓存清理器、解析器分派和执行 Future 公共入口。
#[tokio::test]
async fn registry_and_execution_overloads_share_the_real_flow_bus_state() {
    let bus = FlowBus::new();
    bus.register_arc(
        "a",
        Arc::new(cmp(|ctx| async move {
            ctx.set_data("seen", json!(true));
            Ok(Value::Null)
        })),
    );
    assert!(bus.contain_node("a"));

    // 没有 node_type 元数据的宿主管理组件必须明确失败。
    assert!(matches!(
        bus.add_managed_node("untyped", Arc::new(cmp(|_| async { Ok(Value::Null) }))),
        Err(LiteflowError::NodeBuild(message))
            if message == "node type is null for node[untyped]"
    ));

    bus.register_fallback(
        "fallback",
        NodeTypeEnum::Common,
        cmp(|_| async { Ok(Value::Null) }),
    )
    .unwrap();
    assert!(bus.contains_fallback(NodeTypeEnum::Common));

    let normalized = ElRegexUtil::normalize("THEN(a)");
    let el_md5 = format!("{:x}", Md5::digest(normalized.as_bytes()));
    assert!(matches!(
        bus.add_chain_anonymous("bad_md5", &normalized, "wrong".to_string()),
        Err(LiteflowError::Parse(message))
            if message == "anonymous chain[bad_md5] EL MD5 mismatch"
    ));
    bus.add_chain_anonymous("anonymous", &normalized, el_md5.clone())
        .unwrap();
    assert_eq!(
        bus.get_chain_id_by_el_md5(&el_md5).as_deref(),
        Some("anonymous")
    );

    let cleaner = bus.chain_cache_cleaner();
    cleaner("anonymous");
    assert!(!bus.contains_chain("anonymous"));
    assert!(bus.get_chain_id_by_el_md5(&el_md5).is_none());

    assert_eq!(
        bus.refresh_flow_meta_data(
            FlowParserTypeEnum::TypeElXml,
            "<flow><chain id=\"xml_chain\">THEN(a)</chain></flow>",
        )
        .unwrap(),
        vec!["xml_chain"]
    );
    assert_eq!(
        bus.refresh_flow_meta_data(
            FlowParserTypeEnum::TypeElYml,
            "flow:\n  chain:\n    - id: yml_chain\n      body: THEN(a)\n",
        )
        .unwrap(),
        vec!["yml_chain"]
    );
    assert!(
        bus.refresh_flow_meta_data(FlowParserTypeEnum::TypeJson, "{}")
            .unwrap()
            .is_empty()
    );
    assert!(FlowBus::validate_el("THEN(a)").is_ok());
    assert!(FlowBus::validate_el("THEN(").is_err());

    let handle = bus
        .execute_future_with_option(
            "xml_chain",
            Value::Null,
            ExecuteOption::of().request_id("future-request"),
        )
        .unwrap();
    let response = handle.await.unwrap();
    assert!(response.is_success());
    assert_eq!(response.get_request_id(), "future-request");

    let weak_cleaner = {
        let temporary = FlowBus::new();
        temporary.chain_cache_cleaner()
    };
    weak_cleaner("already-dropped");
}

/// 验证 Java 脚本节点重载入口对全部标准脚本类型使用同一编译主干。
///
/// 对应 Java: `FlowBus#addScriptNodeAndCompile` 与 `#compileScriptNode`。
#[test]
fn script_node_compile_overloads_preserve_all_java_script_kinds() {
    let bus = FlowBus::new();
    bus.add_script_node_and_compile(
        "common_script",
        Some("普通脚本"),
        NodeTypeEnum::Script,
        "()",
        "rhai",
    )
    .unwrap();
    bus.compile_script_node(
        "boolean_script",
        None,
        NodeTypeEnum::BooleanScript,
        "true",
        "rhai",
    )
    .unwrap();
    bus.compile_script_node("if_script", None, NodeTypeEnum::IfScript, "true", "rhai")
        .unwrap();
    bus.compile_script_node(
        "switch_script",
        None,
        NodeTypeEnum::SwitchScript,
        r#""common_script""#,
        "rhai",
    )
    .unwrap();
    for (node_id, node_type) in [
        ("for_script", NodeTypeEnum::ForScript),
        ("while_script", NodeTypeEnum::WhileScript),
        ("break_script", NodeTypeEnum::BreakScript),
    ] {
        bus.compile_script_node(node_id, None, node_type, "1", "rhai")
            .unwrap();
    }

    for node_id in [
        "common_script",
        "boolean_script",
        "if_script",
        "switch_script",
        "for_script",
        "while_script",
        "break_script",
    ] {
        assert!(bus.contains_node(node_id));
    }
    assert!(matches!(
        bus.add_script_node(
            "not_script",
            None,
            NodeTypeEnum::Common,
            "()",
            "rhai"
        ),
        Err(LiteflowError::NodeTypeError {
            node,
            expect,
            actual
        }) if node.is_empty()
            && expect == "script node type"
            && actual == NodeTypeEnum::Common.get_code()
    ));
}
