use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use liteflow_core::flow::parallel::CompletableFutureExpand;
use liteflow_core::lifecycle::ChainCacheLifeCycle;
use liteflow_core::log::LFLoggerManager;
use liteflow_core::meta::LiteflowMetaOperator;
use liteflow_core::parser::{FlowParserProvider, ParserClassNameSpi};
use liteflow_core::thread::ExecutorService;
use liteflow_core::util::{
    CopyOnWriteHashMap, LOGOPrinter, LiteFlowExecutorPoolShutdown, LiteflowContextRegexMatcher,
    QlExpressUtils, SerialsUtil,
};
use liteflow_core::{FlowBus, FlowParserTypeEnum, cmp};
use serde_json::{Value, json};

#[test]
fn utility_objects_execute_java_equivalent_algorithms() {
    assert!(QlExpressUtils::check_variable_name("$order_2"));
    assert!(!QlExpressUtils::check_variable_name("2order"));
    assert!(QlExpressUtils::parse_el("THEN(a, b)").is_ok());

    let map = CopyOnWriteHashMap::new(HashMap::from([("a", 1)]));
    let snapshot = map.snapshot();
    map.insert("b", 2);
    assert_eq!(snapshot.len(), 1);
    assert_eq!(map.get(&"b"), Some(2));
    let cloned = map.clone();
    map.remove(&"a");
    assert_eq!(cloned.get(&"a"), Some(1));

    let mut contexts = vec![
        ("order".to_string(), json!({"customer": {"name": "old"}})),
        ("fallback".to_string(), json!({"name": "fallback"})),
    ];
    assert_eq!(
        LiteflowContextRegexMatcher::search_context(&contexts, "customer.name"),
        Some(json!("old"))
    );
    assert!(LiteflowContextRegexMatcher::search_and_set_context(
        &mut contexts,
        "customer.setName",
        &[json!("new")],
    ));
    assert_eq!(contexts[0].1["customer"]["name"], json!("new"));

    assert_eq!(SerialsUtil::from10_to32("0", 4).unwrap(), "2222");
    assert_eq!(SerialsUtil::from10_to24("0", 4).unwrap(), "BBBB");
    assert_eq!(SerialsUtil::get_uuid().len(), 32);
    assert_eq!(SerialsUtil::generate_short_uuid().len(), 6);
    assert_eq!(SerialsUtil::generate_file_uuid().len(), 8);
    assert!(LOGOPrinter::logo().contains(env!("CARGO_PKG_VERSION")));
}

#[tokio::test]
async fn executor_shutdown_future_timeout_and_logger_context_are_real() {
    let executor = Arc::new(ExecutorService::new(1, 1, 1, "s8"));
    assert!(LiteFlowExecutorPoolShutdown::destroy(executor.clone(), Duration::from_secs(1)).await);
    assert!(executor.is_shutdown());

    let value = CompletableFutureExpand::complete_on_timeout(
        async {
            tokio::time::sleep(Duration::from_millis(30)).await;
            42
        },
        Duration::from_millis(1),
        7,
    )
    .await;
    assert_eq!(value, 7);

    let request_id = LFLoggerManager::scope_request_id("rid-s8", async {
        LFLoggerManager::get_logger("s8").info("inside task");
        LFLoggerManager::get_request_id()
    })
    .await;
    assert_eq!(request_id.as_deref(), Some("rid-s8"));
}

struct TestParserSpi;

impl ParserClassNameSpi for TestParserSpi {
    fn get_spi_class_name(&self) -> &str {
        "tests::TestParser"
    }
}

#[tokio::test]
async fn meta_operator_and_chain_cache_lifecycle_drive_flow_bus() {
    assert_eq!(TestParserSpi.get_spi_class_name(), "tests::TestParser");

    let cleaned = Arc::new(Mutex::new(Vec::<String>::new()));
    let cleaned_for_hook = cleaned.clone();
    let cache = Arc::new(ChainCacheLifeCycle::new(
        1,
        Arc::new(move |chain_id| {
            cleaned_for_hook.lock().unwrap().push(chain_id.to_string());
        }),
    ));
    let bus = FlowBus::new();
    bus.register_chain_execute_hook(cache);
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    bus.register("b", cmp(|_| async { Ok(Value::Null) }));
    let parser_provider = FlowParserProvider::new(bus.clone());
    parser_provider.register_class_parser_spi(
        &TestParserSpi,
        FlowParserTypeEnum::TypeElJson,
        Arc::new(|| Ok(r#"{"flow":{"chain":[{"id":"spiChain","body":"THEN(a)"}]}}"#.to_string())),
    );
    let spi_parser = parser_provider.lookup("el_json:tests::TestParser").unwrap();
    assert_eq!(spi_parser.parse_main(&[]).unwrap(), vec!["spiChain"]);
    bus.add_chain("chainA", "THEN(a, a)").unwrap();
    bus.add_chain("chainB", "THEN(b)").unwrap();
    let metadata = LiteflowMetaOperator::new(bus.clone());

    assert_eq!(metadata.get_nodes("chainA").len(), 2);
    assert_eq!(metadata.get_nodes_by_id("chainA", "a").len(), 2);
    // Parser SPI 构建的 spiChain 与直接构建的 chainA 都真实包含节点 a。
    assert_eq!(metadata.get_chains_contains_node_id("a").len(), 2);
    assert!(bus.execute("chainA").await.is_success());
    assert!(bus.execute("chainB").await.is_success());
    assert_eq!(cleaned.lock().unwrap().as_slice(), ["chainA"]);

    bus.register_script("hotScript", "rhai", r#"data["version"] = 1;"#)
        .unwrap();
    bus.add_chain("hotChain", "THEN(hotScript)").unwrap();
    assert_eq!(
        bus.execute("hotChain").await.data("version"),
        Some(json!(1))
    );
    metadata
        .reload_script("hotScript", r#"data["version"] = 2;"#)
        .unwrap();
    assert_eq!(
        bus.execute("hotChain").await.data("version"),
        Some(json!(2))
    );
}
