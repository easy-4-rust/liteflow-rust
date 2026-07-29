//! FlowExecutor 首次执行初始化与 Java 公共异步重载语义验收。

use std::sync::Arc;

use liteflow_core::{
    ExecutorBuilder, ExecutorHelper, ExecutorService, FlowBus, FlowExecutor, FlowParserTypeEnum,
    LiteflowConfig, NodeTypeEnum, ParseModeEnum, cmp,
};
use serde::Serialize;
use serde::ser::{Error as SerError, Serializer};
use serde_json::{Value, json};

struct RejectSerialize;

struct ClosedExecutorBuilder;

impl ExecutorBuilder for ClosedExecutorBuilder {
    fn build_executor(&self) -> Arc<ExecutorService> {
        Arc::new(ExecutorService::new(1, 1, 1, "closed-flow-executor"))
    }
}

impl Serialize for RejectSerialize {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Err(S::Error::custom("intentional serialization failure"))
    }
}

/// 验证 Java doExecute 在首次执行前通过 FlowBus.needInit 原子加载规则。
///
/// 对应 Java: `FlowExecutor#doExecute`。
#[tokio::test]
async fn first_body_execution_lazily_loads_registered_rule_source_once() {
    let bus = FlowBus::new();
    bus.register("body", cmp(|_| async { Ok(Value::Null) }));
    let mut config = LiteflowConfig::default();
    config.set_parse_mode(ParseModeEnum::ParseAllOnFirstExec);
    config.set_rule_source(Some("el_json:tests.LazyBodyParser".to_string()));
    let executor = FlowExecutor::new_isolated(bus.clone(), config);
    executor.register_class_parser(
        "tests.LazyBodyParser",
        FlowParserTypeEnum::TypeElJson,
        Arc::new(|| {
            Ok(r#"{"flow":{"chain":[{"id":"lazy_body_chain","body":"THEN(body)"}]}}"#.to_string())
        }),
    );

    let response = executor.execute("lazy_body_chain").await;
    assert!(response.is_success(), "{}", response.message);
    assert!(bus.contain_chain("lazy_body_chain"));

    // init_stat 已领取，第二次执行直接复用已发布 Chain。
    assert!(executor.execute("lazy_body_chain").await.is_success());
}

/// 验证路由入口也在查询 Chain 前初始化，并让命中的主体复用同一 requestId。
///
/// 对应 Java: `FlowExecutor#doExecuteWithRoute`。
#[tokio::test]
async fn first_route_execution_initializes_rules_before_namespace_query() {
    let bus = FlowBus::new();
    bus.add_node(
        "route",
        None,
        NodeTypeEnum::Boolean,
        Arc::new(cmp(|_| async { Ok(Value::Bool(true)) })),
    )
    .unwrap();
    bus.register(
        "body",
        cmp(|ctx| async move {
            ctx.set_data("executed", json!(true));
            Ok(Value::Null)
        }),
    );
    let mut config = LiteflowConfig::default();
    config.set_parse_mode(ParseModeEnum::ParseOneOnFirstExec);
    config.set_rule_source(Some("el_json:tests.LazyRouteParser".to_string()));
    let executor = FlowExecutor::new_isolated(bus, config);
    executor.register_class_parser(
        "tests.LazyRouteParser",
        FlowParserTypeEnum::TypeElJson,
        Arc::new(|| {
            Ok(
                r#"{"flow":{"chain":[{"id":"lazy_route","namespace":"lazy","route":"route","body":"THEN(body)"}]}}"#
                    .to_string(),
            )
        }),
    );

    let responses = executor
        .execute_route_chain_with_rid(Some("lazy"), Value::Null, "lazy-request")
        .await
        .unwrap();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].get_request_id(), "lazy-request");
    assert_eq!(responses[0].data("executed"), Some(json!(true)));
}

/// 验证初始化失败进入失败响应，同时保留调用方 requestId。
#[tokio::test]
async fn lazy_initialization_failure_is_an_observable_failed_response() {
    let bus = FlowBus::new();
    let mut config = LiteflowConfig::default();
    config.set_parse_mode(ParseModeEnum::ParseAllOnFirstExec);
    config.set_rule_source(Some(
        "/path/that/does/not/exist/liteflow-lazy-rule.json".to_string(),
    ));
    let executor = FlowExecutor::new_isolated(bus, config);
    let response = executor
        .execute_with_rid("missing_chain", Value::Null, "failed-init", Vec::new())
        .await;
    assert!(!response.is_success());
    assert_eq!(response.get_request_id(), "failed-init");
    assert!(!response.message.is_empty());
}

/// 验证 Future 公共重载进入真实主执行器，序列化失败按 Java null 入参继续执行。
#[tokio::test]
async fn future_overloads_and_serialization_fallback_execute_real_chain() {
    let bus = FlowBus::new();
    bus.register(
        "inspect",
        cmp(|ctx| async move {
            let input_is_null = ctx
                .request_data::<Value>()
                .is_some_and(|input| input.is_null());
            ctx.set_data("input_is_null", json!(input_is_null));
            Ok(Value::Null)
        }),
    );
    bus.add_chain("future_chain", "THEN(inspect)").unwrap();
    let executor = FlowExecutor::new(bus);

    let serialized = executor
        .execute_with_data("future_chain", RejectSerialize)
        .await;
    assert_eq!(serialized.data("input_is_null"), Some(json!(true)));

    let response = executor
        .execute2_future("future_chain", Value::Null, None)
        .unwrap()
        .await
        .unwrap();
    assert!(response.is_success());

    let response = executor
        .execute2_future_with_rid("future_chain", Value::Null, "future-request", Vec::new())
        .unwrap()
        .await
        .unwrap();
    assert_eq!(response.get_request_id(), "future-request");
}

/// 验证 Java 仅在 PARSE_ONE_ON_FIRST_EXEC 下启用 Chain 缓存并校验容量。
///
/// 对应 Java: `FlowExecutor#initChainCache`。
#[test]
fn chain_cache_initialization_obeys_parse_mode_and_capacity_contract() {
    let mut ignored = LiteflowConfig::default();
    ignored.set_chain_cache_enabled(true);
    ignored.set_chain_cache_capacity(0);
    assert!(
        FlowExecutor::new_isolated(FlowBus::new(), ignored)
            .init(true)
            .is_ok(),
        "非 PARSE_ONE_ON_FIRST_EXEC 模式必须忽略 Chain 缓存"
    );

    let mut invalid = LiteflowConfig::default();
    invalid.set_parse_mode(ParseModeEnum::ParseOneOnFirstExec);
    invalid.set_chain_cache_enabled(true);
    invalid.set_chain_cache_capacity(0);
    let error = FlowExecutor::new_isolated(FlowBus::new(), invalid)
        .init(true)
        .expect_err("Java 要求缓存容量大于零");
    assert!(error.to_string().contains("greater than 0"));

    let mut valid = LiteflowConfig::default();
    valid.set_parse_mode(ParseModeEnum::ParseOneOnFirstExec);
    valid.set_chain_cache_enabled(true);
    valid.set_chain_cache_capacity(1);
    FlowExecutor::new_isolated(FlowBus::new(), valid)
        .init(true)
        .unwrap();
}

/// 验证多类型开关决定逐文件解析或拒绝混合 Parser。
///
/// 对应 Java: `FlowExecutor#init(boolean)` 的 parserNameSet 分支。
#[test]
fn mixed_rule_formats_require_support_multiple_type() {
    let directory = tempfile::tempdir().unwrap();
    let json_path = directory.path().join("one.json");
    let xml_path = directory.path().join("two.xml");
    std::fs::write(
        &json_path,
        r#"{"flow":{"chain":[{"id":"json_chain","body":"THEN(body)"}]}}"#,
    )
    .unwrap();
    std::fs::write(
        &xml_path,
        r#"<flow><chain id="xml_chain"><body>THEN(body)</body></chain></flow>"#,
    )
    .unwrap();
    let rule_source = format!("{},{}", json_path.display(), xml_path.display());

    let rejected_bus = FlowBus::new();
    rejected_bus.register("body", cmp(|_| async { Ok(Value::Null) }));
    let mut rejected = LiteflowConfig::default();
    rejected.set_rule_source(Some(rule_source.clone()));
    assert!(matches!(
        FlowExecutor::new_isolated(rejected_bus, rejected).init(true),
        Err(liteflow_core::LiteflowError::MultipleParsers(_))
    ));

    let accepted_bus = FlowBus::new();
    accepted_bus.register("body", cmp(|_| async { Ok(Value::Null) }));
    let mut accepted = LiteflowConfig::default();
    accepted.set_rule_source(Some(rule_source));
    accepted.set_support_multiple_type(true);
    FlowExecutor::new_isolated(accepted_bus.clone(), accepted)
        .init(true)
        .unwrap();
    assert!(accepted_bus.contain_chain("json_chain"));
    assert!(accepted_bus.contain_chain("xml_chain"));
}

/// 验证启动初始化把真实规则文件交给 MonitorFile，并能显式清理后台任务。
#[tokio::test]
async fn startup_monitor_uses_real_rule_path_and_is_cleanable() {
    let directory = tempfile::tempdir().unwrap();
    let rule_path = directory.path().join("monitored.json");
    std::fs::write(
        &rule_path,
        r#"{"flow":{"chain":[{"id":"monitored_chain","body":"THEN(body)"}]}}"#,
    )
    .unwrap();
    let bus = FlowBus::new();
    bus.register("body", cmp(|_| async { Ok(Value::Null) }));
    let mut config = LiteflowConfig::default();
    config.set_rule_source(Some(rule_path.to_string_lossy().into_owned()));
    config.set_enable_monitor_file(true);
    let executor = FlowExecutor::new_isolated(bus.clone(), config);

    executor.init(true).unwrap();
    assert!(bus.contain_chain("monitored_chain"));
    bus.clean_monitor_file().unwrap();
}

/// 验证存在真实监听文件但没有 Tokio runtime 时返回可诊断初始化错误。
#[test]
fn startup_monitor_requires_active_tokio_runtime() {
    let directory = tempfile::tempdir().unwrap();
    let rule_path = directory.path().join("monitor-without-runtime.json");
    std::fs::write(&rule_path, r#"{"flow":{"chain":[]}}"#).unwrap();
    let mut config = LiteflowConfig::default();
    config.set_rule_source(Some(rule_path.to_string_lossy().into_owned()));
    config.set_enable_monitor_file(true);
    let error = FlowExecutor::new_isolated(FlowBus::new(), config)
        .init(true)
        .expect_err("无 Tokio runtime 时不能创建文件监听任务");
    assert!(error.to_string().contains("active Tokio runtime"));
}

/// 验证仅包含分隔符的 ruleSource 与 Java 一样属于配置错误。
#[test]
fn empty_rule_path_list_is_not_silently_accepted() {
    let mut config = LiteflowConfig::default();
    config.set_rule_source(Some(" , ; ".to_string()));
    let error = FlowExecutor::new_isolated(FlowBus::new(), config)
        .init(true)
        .expect_err("空路径列表不能被当成动态构建模式");
    assert!(error.to_string().contains("parse error"));
}

/// 验证 route 返回非布尔值或异常时都不匹配，并覆盖空 requestId 自动生成。
#[tokio::test]
async fn route_non_boolean_and_error_results_are_filtered_like_java() {
    for (node_id, component) in [
        (
            "non_boolean",
            Arc::new(cmp(|_| async { Ok(Value::String("true".to_string())) }))
                as Arc<dyn liteflow_core::NodeComponent>,
        ),
        (
            "route_error",
            Arc::new(cmp(|_| async {
                Err(liteflow_core::LiteflowError::Custom(
                    "route failed".to_string(),
                ))
            })) as Arc<dyn liteflow_core::NodeComponent>,
        ),
    ] {
        let bus = FlowBus::new();
        bus.add_node(node_id, None, NodeTypeEnum::Boolean, component)
            .unwrap();
        bus.register("body", cmp(|_| async { Ok(Value::Null) }));
        bus.reload_chain_with_route("route_chain", "THEN(body)", Some(node_id))
            .unwrap();
        let executor = FlowExecutor::new(bus);
        let error = executor
            .execute_route_chain_with_rid(None, RejectSerialize, "")
            .await
            .expect_err("非 true route 不应产生匹配 Chain");
        assert!(matches!(
            error,
            liteflow_core::LiteflowError::NoMatchedRouteChain(_)
        ));
    }
}

/// 验证未超时分支返回真实成功响应。
#[tokio::test]
async fn timeout_entry_returns_fast_chain_response_before_deadline() {
    let bus = FlowBus::new();
    bus.register("fast", cmp(|_| async { Ok(Value::Null) }));
    bus.add_chain("fast_chain", "THEN(fast)").unwrap();
    let response = FlowExecutor::new(bus)
        .execute_timeout("fast_chain", Value::Null, std::time::Duration::from_secs(1))
        .await;
    assert!(response.is_success(), "{}", response.message);
}

/// 验证主执行器在任务提交前已关闭时，Future 返回失败响应而不丢失关联信息。
#[tokio::test]
async fn closed_main_executor_becomes_failed_liteflow_response() {
    const EXECUTOR_CLASS: &str = "tests.ClosedFlowMainExecutor";
    let helper = ExecutorHelper::load_instance();
    helper.register_executor_builder(EXECUTOR_CLASS, Arc::new(ClosedExecutorBuilder));
    let service = helper.build_main_executor(Some(EXECUTOR_CLASS)).unwrap();
    service.shutdown();

    let bus = FlowBus::new();
    bus.register("body", cmp(|_| async { Ok(Value::Null) }));
    bus.add_chain("closed_executor_chain", "THEN(body)")
        .unwrap();
    let response = FlowExecutor::new(bus)
        .execute_future_with_executor(
            "closed_executor_chain",
            Value::Null,
            liteflow_core::ExecuteOption::of().request_id("closed-request"),
            Some(EXECUTOR_CLASS),
        )
        .unwrap()
        .await
        .unwrap();
    assert!(!response.is_success());
    assert_eq!(response.get_request_id(), "closed-request");
    assert!(response.message.contains("shut down"));
}

/// 验证匿名 EL 构建失败返回完整失败响应。
#[tokio::test]
async fn anonymous_el_with_missing_node_returns_build_failure_response() {
    let response = FlowExecutor::new(FlowBus::new())
        .execute_with_el("THEN(missing_node)")
        .await;
    assert!(!response.is_success());
    assert!(response.message.contains("missing_node"));
}
