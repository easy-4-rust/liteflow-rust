use std::any::Any;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use liteflow_core::core::DeclComponent;
use liteflow_core::core::proxy::{
    DeclWarpBean, LiteFlowMethodBean, LiteFlowProxyUtil, MethodWrapBean, ParameterWrapBean,
};
use liteflow_core::enums::{LiteFlowMethodEnum, NodeTypeEnum};
use liteflow_core::spi::{
    DeclComponentParser, DeclComponentParserHolder, SpiFactoryInitializing, SpiPriority,
};
use liteflow_core::{CmpContext, FlowBus, LiteflowError};
use serde_json::{Value, json};

struct StockFact {
    available: bool,
}

struct RawDeclComponent {
    attempts: Arc<AtomicUsize>,
}

#[async_trait]
impl DeclComponent for RawDeclComponent {
    async fn call(&self, method: &str, context: &CmpContext) -> Result<Value, LiteflowError> {
        match method {
            "loadStock" => {
                let stock = context.bean::<StockFact>("stockFact").ok_or_else(|| {
                    LiteflowError::ParameterFact("typed fact missing".to_string())
                })?;
                context.set_data("available", json!(stock.available));
                Ok(Value::Null)
            }
            "retryOnce" => {
                let attempt = self.attempts.fetch_add(1, Ordering::SeqCst);
                if attempt == 0 {
                    Err(LiteflowError::Parse("temporary".to_string()))
                } else {
                    context.set_data("retried", json!(true));
                    Ok(Value::Null)
                }
            }
            other => Err(LiteflowError::CmpDefine(format!(
                "unknown raw method[{other}]"
            ))),
        }
    }
}

fn method(
    name: &str,
    node_type: NodeTypeEnum,
    retry: Option<usize>,
    retry_for: Vec<String>,
    parameters: Vec<ParameterWrapBean>,
) -> MethodWrapBean {
    MethodWrapBean::new(
        LiteFlowMethodBean::new(name, LiteFlowMethodEnum::Process),
        LiteFlowMethodEnum::Process,
        node_type,
        retry,
        retry_for,
        parameters,
    )
}

fn warp(methods: Vec<MethodWrapBean>) -> DeclWarpBean {
    DeclWarpBean::new(
        "inventory",
        "库存声明式组件",
        NodeTypeEnum::Common,
        Arc::new(RawDeclComponent {
            attempts: Arc::new(AtomicUsize::new(0)),
        }),
        "tests::RawDeclComponent",
        methods,
    )
}

#[tokio::test]
async fn proxy_registration_validates_fact_and_executes_real_decl_method() {
    let load_stock = method(
        "loadStock",
        NodeTypeEnum::Common,
        None,
        Vec::new(),
        vec![ParameterWrapBean::new(
            "Arc<StockFact>",
            Some("stockFact"),
            1,
        )],
    );
    let declaration = warp(vec![load_stock]);
    assert!(LiteFlowProxyUtil::is_declare_cmp(&declaration));

    let bus = FlowBus::new();
    bus.try_register_decl_warp(declaration).unwrap();
    bus.add_chain("inventoryChain", "THEN(inventory.loadStock)")
        .unwrap();

    let beans: Vec<(String, Arc<dyn Any + Send + Sync>)> = vec![(
        "stockFact".to_string(),
        Arc::new(StockFact { available: true }),
    )];
    let response = bus.execute_with("inventoryChain", Value::Null, beans).await;

    assert!(response.is_success(), "{}", response.message);
    assert_eq!(response.data("available"), Some(json!(true)));
    assert_eq!(response.steps[0].node_name, "库存声明式组件");
}

#[tokio::test]
async fn proxy_rejects_missing_fact_before_typed_dispatch() {
    let bus = FlowBus::new();
    bus.try_register_decl_warp(warp(vec![method(
        "loadStock",
        NodeTypeEnum::Common,
        None,
        Vec::new(),
        vec![ParameterWrapBean::new(
            "Arc<StockFact>",
            Some("stockFact"),
            1,
        )],
    )]))
    .unwrap();
    bus.add_chain("missingFact", "THEN(inventory.loadStock)")
        .unwrap();

    let response = bus.execute("missingFact").await;

    assert!(!response.is_success());
    assert!(response.message.contains("fact bean[stockFact] not found"));
}

#[tokio::test]
async fn proxy_method_retry_metadata_drives_real_node_retry() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let declaration = DeclWarpBean::new(
        "inventory",
        "库存声明式组件",
        NodeTypeEnum::Common,
        Arc::new(RawDeclComponent {
            attempts: attempts.clone(),
        }),
        "tests::RawDeclComponent",
        vec![method(
            "retryOnce",
            NodeTypeEnum::Common,
            Some(1),
            vec!["java.text.ParseException".to_string()],
            Vec::new(),
        )],
    );
    let bus = FlowBus::new();
    bus.try_register_decl_warp(declaration).unwrap();
    bus.add_chain("retryDecl", "THEN(inventory.retryOnce)")
        .unwrap();

    let response = bus.execute("retryDecl").await;

    assert!(response.is_success(), "{}", response.message);
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    assert_eq!(response.data("retried"), Some(json!(true)));
}

#[test]
fn proxy_rejects_mixed_node_types_and_unknown_methods_at_build_time() {
    let mixed = warp(vec![
        method("common", NodeTypeEnum::Common, None, Vec::new(), Vec::new()),
        method(
            "boolean",
            NodeTypeEnum::Boolean,
            None,
            Vec::new(),
            Vec::new(),
        ),
    ]);
    let error = match LiteFlowProxyUtil::proxy2_node_component(mixed) {
        Ok(_) => panic!("mixed node types must be rejected"),
        Err(error) => error,
    };
    assert!(matches!(error, LiteflowError::ComponentProxyError(_)));

    let bus = FlowBus::new();
    bus.try_register_decl_warp(warp(vec![method(
        "known",
        NodeTypeEnum::Common,
        None,
        Vec::new(),
        Vec::new(),
    )]))
    .unwrap();
    let error = bus
        .add_chain("unknownMethod", "THEN(inventory.unknown)")
        .expect_err("unknown declarative methods must fail while building the chain");
    assert!(error.to_string().contains("method[unknown] not registered"));

    assert!(LiteFlowProxyUtil::is_cglib_proxy_class(
        "com.demo.Inventory$$EnhancerBySpringCGLIB"
    ));
    assert_eq!(
        LiteFlowProxyUtil::get_user_class("com.demo.Inventory$$EnhancerBySpringCGLIB"),
        "com.demo.Inventory"
    );
}

#[test]
fn proxy_metadata_beans_preserve_java_mutation_contract() {
    let java_method_names = [
        "process",
        "processSwitch",
        "processBoolean",
        "processFor",
        "processIterator",
        "isAccess",
        "isEnd",
        "isContinueOnError",
        "getNodeExecutorClass",
        "onSuccess",
        "onError",
        "beforeProcess",
        "afterProcess",
        "getDisplayName",
        "rollback",
    ];
    for (index, method_name) in java_method_names.into_iter().enumerate() {
        let method = LiteFlowMethodEnum::get_enum_by_method_name(method_name)
            .expect("all Java LiteFlowMethodEnum names must be mapped");
        assert_eq!(method.get_method_name(), method_name);
        assert_eq!(method.is_main_method(), index < 5);
    }

    let mut method_bean = LiteFlowMethodBean::new("before", LiteFlowMethodEnum::BeforeProcess);
    method_bean.set_method_name("process");
    method_bean.set_method(LiteFlowMethodEnum::Process);
    assert_eq!(method_bean.method_name(), "process");
    assert_eq!(method_bean.method(), LiteFlowMethodEnum::Process);

    let mut parameter = ParameterWrapBean::new("Arc<OldFact>", Some("oldFact"), 2);
    parameter.set_parameter_type("Arc<StockFact>");
    parameter.set_fact(Some("stockFact"));
    parameter.set_index(1);
    assert_eq!(parameter.parameter_type(), "Arc<StockFact>");
    assert_eq!(parameter.fact(), Some("stockFact"));
    assert_eq!(parameter.index(), 1);

    let mut declaration = warp(vec![method(
        "loadStock",
        NodeTypeEnum::Common,
        None,
        Vec::new(),
        vec![parameter],
    )]);
    declaration.set_node_id("inventoryV2");
    declaration.set_node_name("库存 V2");
    declaration.set_raw_clazz("tests::InventoryV2");
    assert_eq!(declaration.node_id(), "inventoryV2");
    assert_eq!(declaration.node_name(), "库存 V2");
    assert_eq!(declaration.raw_clazz(), "tests::InventoryV2");
}

struct CountingDeclComponentParser {
    calls: Arc<AtomicUsize>,
}

impl SpiPriority for CountingDeclComponentParser {
    fn priority(&self) -> i32 {
        1
    }
}

impl DeclComponentParser for CountingDeclComponentParser {
    fn parse_decl_bean(
        &self,
        decl_warp_bean: DeclWarpBean,
    ) -> Result<Vec<DeclWarpBean>, LiteflowError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(vec![decl_warp_bean])
    }
}

#[test]
fn declaration_parser_holder_and_factory_initializing_drive_registration() {
    SpiFactoryInitializing::clean();
    SpiFactoryInitializing::load_init();
    assert_eq!(
        DeclComponentParserHolder::load_decl_component_parser().priority(),
        2
    );

    let calls = Arc::new(AtomicUsize::new(0));
    DeclComponentParserHolder::register(Arc::new(CountingDeclComponentParser {
        calls: calls.clone(),
    }));
    let bus = FlowBus::new();
    bus.try_register_decl_warp(warp(vec![method(
        "known",
        NodeTypeEnum::Common,
        None,
        Vec::new(),
        Vec::new(),
    )]))
    .unwrap();

    // Rust 测试默认并行；同一测试二进制中的其他注册也可能在本 Holder 有效期内
    // 经过该透传解析器，因此这里只验证至少一次真实调用。
    assert!(calls.load(Ordering::SeqCst) >= 1);
    SpiFactoryInitializing::clean();
}
