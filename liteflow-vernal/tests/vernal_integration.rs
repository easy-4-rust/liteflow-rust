//! Vernal 生命周期、IoC 解析和 Web 适配的真实集成测试。

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use liteflow_core::aop::ICmpAroundAspect;
use liteflow_core::core::DeclComponent;
use liteflow_core::core::cmp;
use liteflow_core::core::proxy::{DeclWarpBean, LiteFlowMethodBean, MethodWrapBean};
use liteflow_core::enums::{LiteFlowMethodEnum, NodeTypeEnum};
use liteflow_core::script::ScriptBeanManager;
use liteflow_core::script::proxy::{ScriptBeanProxy, ScriptMethodProxy};
use liteflow_core::spi::{
    CmpAroundAspectHolder, ContextAwareHolder, ContextCmpInitHolder, PathContentParserHolder,
};
use liteflow_core::{CmpContext, LiteflowError, NodeComponent};
use liteflow_vernal::process::LiteflowScannerProcessStepFactory;
use liteflow_vernal::process::enums::LiteflowScannerProcessStepEnum;
use liteflow_vernal::process::holder::{SpringCmpAroundAspectHolder, SpringNodeIdHolder};
use liteflow_vernal::{
    LiteflowComponentRegistration, LiteflowConfig, LiteflowConfigGetter, LiteflowExecuteResponse,
    LiteflowExecutorInit, LiteflowMainAutoConfiguration, LiteflowMonitorProperty,
    LiteflowParseMode, LiteflowProperty, LiteflowPropertyAutoConfiguration, LiteflowRuleFormat,
    LiteflowRuntime, LiteflowSpiInit, LiteflowVernalModule, VernalAware, VernalCmpAroundAspect,
    VernalComponentScanner, VernalContextCmpInit, VernalDeclBeanDefinition,
    VernalDeclComponentParser, VernalLiteflowComponentSupport, VernalPathContentParser,
};
use serde_json::{Value, json};
use vernal_context::{ApplicationContext, VernalApplicationBuilder};

const INLINE_RULE: &str = r#"{
  "flow": {
    "chain": [
      {"id": "vernal_chain", "body": "vernal_component"}
    ]
  }
}"#;

const MANAGED_COMPONENT_RULE: &str = r#"{
  "flow": {
    "chain": [
      {"id": "managed_component_chain", "body": "THEN(managed_ok, managed_error)"}
    ]
  }
}"#;

const DECLARATIVE_COMPONENT_RULE: &str = r#"{
  "flow": {
    "chain": [
      {"id": "declarative_component_chain", "body": "THEN(vernal_decl.process)"}
    ]
  }
}"#;

#[derive(Default)]
struct ManagedComponentCounts {
    process: AtomicUsize,
}

struct ManagedTestComponent {
    fail: bool,
    name: &'static str,
    counts: Arc<ManagedComponentCounts>,
}

#[liteflow_core::async_trait]
impl NodeComponent for ManagedTestComponent {
    async fn process(&self, _context: &CmpContext) -> Result<Value, LiteflowError> {
        self.counts.process.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            Err(LiteflowError::Custom(
                "managed component failure".to_string(),
            ))
        } else {
            Ok(Value::Null)
        }
    }

    fn node_type(&self) -> Option<NodeTypeEnum> {
        Some(NodeTypeEnum::Common)
    }

    fn name(&self) -> &str {
        self.name
    }
}

struct VernalDeclarativeComponent;

#[liteflow_core::async_trait]
impl DeclComponent for VernalDeclarativeComponent {
    async fn call(&self, method: &str, context: &CmpContext) -> Result<Value, LiteflowError> {
        match method {
            "process" => {
                context.set_data("declarative_result", json!("vernal"));
                Ok(Value::Null)
            }
            other => Err(LiteflowError::CmpDefinition(format!(
                "unknown declarative method[{other}]"
            ))),
        }
    }
}

fn declarative_method(method: LiteFlowMethodEnum) -> MethodWrapBean {
    MethodWrapBean::new(
        LiteFlowMethodBean::new(method.get_method_name(), method),
        method,
        NodeTypeEnum::Common,
        None,
        Vec::new(),
        Vec::new(),
    )
}

fn declarative_component(node_id: &str, methods: Vec<MethodWrapBean>) -> DeclWarpBean {
    DeclWarpBean::new(
        node_id,
        "Vernal 声明式组件",
        NodeTypeEnum::Common,
        Arc::new(VernalDeclarativeComponent),
        std::any::type_name::<VernalDeclarativeComponent>(),
        methods,
    )
}

#[derive(Default)]
struct ManagedAspectCounts {
    before: AtomicUsize,
    after: AtomicUsize,
    success: AtomicUsize,
    error: AtomicUsize,
}

struct ManagedTestAspect {
    counts: Arc<ManagedAspectCounts>,
}

impl ICmpAroundAspect for ManagedTestAspect {
    fn before_process(&self, _context: &CmpContext) {
        self.counts.before.fetch_add(1, Ordering::SeqCst);
    }

    fn after_process(&self, _context: &CmpContext) {
        self.counts.after.fetch_add(1, Ordering::SeqCst);
    }

    fn on_success(&self, _context: &CmpContext) {
        self.counts.success.fetch_add(1, Ordering::SeqCst);
    }

    fn on_error(&self, _context: &CmpContext, _error: &LiteflowError) {
        self.counts.error.fetch_add(1, Ordering::SeqCst);
    }
}

const LAZY_JSON_RULE: &str = r#"{
  "flow": {
    "nodes": {
      "node": [
        {"id": "root_script", "type": "script", "value": "40 + 2"},
        {"id": "child_script", "type": "script", "value": "20 + 1"},
        {"id": "unused_script", "type": "script", "value": "1"}
      ]
    },
    "chain": [
      {"id": "root_chain", "body": "THEN(child_chain, root_script)"},
      {"id": "child_chain", "body": "THEN(child_script)"},
      {"id": "unused_chain", "body": "THEN(unused_script)"}
    ]
  }
}"#;

const LAZY_XML_RULE: &str = r#"<flow>
  <nodes>
    <node id="root_script" type="script">40 + 2</node>
    <node id="child_script" type="script">20 + 1</node>
    <node id="unused_script" type="script">1</node>
  </nodes>
  <chain id="root_chain"><body>THEN(child_chain, root_script)</body></chain>
  <chain id="child_chain"><body>THEN(child_script)</body></chain>
  <chain id="unused_chain"><body>THEN(unused_script)</body></chain>
</flow>"#;

const LAZY_YML_RULE: &str = r#"flow:
  nodes:
    node:
      - id: root_script
        type: script
        value: "40 + 2"
      - id: child_script
        type: script
        value: "20 + 1"
      - id: unused_script
        type: script
        value: "1"
  chain:
    - id: root_chain
      body: "THEN(child_chain, root_script)"
    - id: child_chain
      body: "THEN(child_script)"
    - id: unused_chain
      body: "THEN(unused_script)"
"#;

async fn ready_context() -> Arc<ApplicationContext> {
    let registration = LiteflowComponentRegistration::new("vernal_component", |flow_bus| {
        flow_bus.register(
            "vernal_component",
            cmp(|context| async move {
                let input = context
                    .inner
                    .input
                    .lock()
                    .ok()
                    .and_then(|input| input.get("value").cloned())
                    .unwrap_or(Value::Null);
                context.set_data("observed", input);
                Ok(Value::Null)
            }),
        );
        Ok(())
    });
    let module = LiteflowVernalModule::new(
        LiteflowConfig::new().with_inline_rule(LiteflowRuleFormat::Json, INLINE_RULE),
    )
    .with_component(registration);
    let mut builder = VernalApplicationBuilder::current().unwrap();
    builder.register_module(module).unwrap();
    builder.launch().await.unwrap()
}

/// 验证 VernalAware 的命名注册、类型查询和 register-or-get 使用同一真实对象。
///
/// 对应 Java: `SpringAware#getBean/registerBean/registerOrGet/getBeansOfType/hasBean`。
#[test]
fn vernal_aware_preserves_named_and_typed_bean_contracts() {
    let aware = VernalAware::new();
    let typed = Arc::new(String::from("typed-value"));
    let registered = aware.register_typed_bean("typedBean", Arc::clone(&typed));

    assert!(aware.has_bean("typedBean"));
    assert!(aware.has_bean_type(std::any::type_name::<String>()));
    assert!(Arc::ptr_eq(
        &registered.downcast::<String>().unwrap(),
        &typed
    ));
    let typed_beans = aware
        .get_beans_of_type(Some(std::any::type_name::<String>()))
        .unwrap();
    assert_eq!(typed_beans.len(), 1);
    assert!(typed_beans.contains_key("typedBean"));

    let first = aware.register_or_get("createdBean", &|| Arc::new(41_i32));
    let second = aware.register_or_get("createdBean", &|| Arc::new(42_i32));
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(*second.downcast::<i32>().unwrap(), 41);

    aware.register_bean("overriddenBean", Arc::new(1_i64));
    aware.register_bean("overriddenBean", Arc::new(2_i64));
    assert_eq!(
        *aware
            .get_bean("overriddenBean")
            .unwrap()
            .downcast::<i64>()
            .unwrap(),
        2
    );
    assert_eq!(aware.get_beans_of_type(None).unwrap().len(), 3);
    assert_eq!(aware.priority(), 1);
}

/// 验证 Rust 编译期声明元数据经过真实解析 SPI 后才注册到命名容器，未包含
/// `@LiteflowMethod` 等价元数据的源定义不会伪装成已完成声明组件。
///
/// 对应 Java: `DeclBeanDefinition#postProcessBeanDefinitionRegistry`。
#[test]
fn vernal_decl_bean_definition_filters_parses_and_registers_declarations() {
    liteflow_core::spi::DeclComponentParserHolder::register(Arc::new(
        VernalDeclComponentParser::new(),
    ));
    let aware = VernalAware::new();
    let declaration = declarative_component(
        "definition_decl",
        vec![declarative_method(LiteFlowMethodEnum::Process)],
    );
    let raw_bean = Arc::clone(declaration.raw_bean());
    let registrations = vec![
        LiteflowComponentRegistration::declarative(declaration),
        LiteflowComponentRegistration::declarative(declarative_component(
            "definition_without_method",
            Vec::new(),
        )),
    ];
    let definition = VernalDeclBeanDefinition::new();

    let processed = definition
        .post_process_bean_definition_registry(&registrations, &aware)
        .unwrap();
    definition.post_process_bean_factory();

    assert!(definition.is_bean_factory_post_processed());
    assert_eq!(processed.len(), 1);
    assert_eq!(processed[0].component_id(), "definition_decl");
    let registered = aware
        .get_bean("definition_decl")
        .unwrap()
        .downcast::<DeclWarpBean>()
        .unwrap();
    assert!(Arc::ptr_eq(registered.raw_bean(), &raw_bean));
    assert!(!aware.has_bean("definition_without_method"));
}

/// 验证显式 Vernal 扫描器执行普通注册、延迟托管节点并维护上下文隔离缓存。
///
/// 对应 Java: `ComponentScanner#postProcessBeforeInitialization`、
/// `ComponentScanner#postProcessAfterInitialization` 与 `cleanCache`。
#[test]
fn vernal_component_scanner_processes_real_registrations_and_cache() {
    let counts = Arc::new(ManagedComponentCounts::default());
    let managed_component: Arc<dyn NodeComponent> = Arc::new(ManagedTestComponent {
        fail: false,
        name: "扫描器托管组件",
        counts,
    });
    let regular = LiteflowComponentRegistration::new("scanner_regular", |flow_bus| {
        flow_bus.register("scanner_regular", cmp(|_| async { Ok(Value::Null) }));
        Ok(())
    });
    let scanner = VernalComponentScanner::with_config(
        &LiteflowConfig {
            print_banner: false,
            ..LiteflowConfig::default()
        },
        vec![
            regular.clone(),
            LiteflowComponentRegistration::managed("scanner_managed", managed_component),
        ],
    );
    let flow_bus = liteflow_core::FlowBus::new();

    let before = scanner.post_process_before_initialization(&regular);
    assert_eq!(before.component_id(), regular.component_id());
    let managed_registrations = scanner.scan(&flow_bus).unwrap();

    assert!(flow_bus.contains_node("scanner_regular"));
    assert!(!flow_bus.contains_node("scanner_managed"));
    assert_eq!(managed_registrations.len(), 1);
    assert_eq!(managed_registrations[0].component_id(), "scanner_managed");
    assert!(managed_registrations[0].managed_component().is_some());
    assert_eq!(
        scanner.scanned_component_ids(),
        vec!["scanner_managed".to_string(), "scanner_regular".to_string()]
    );
    assert_eq!(
        scanner.spring_node_id_holder().get_node_id_set(),
        vec!["scanner_managed".to_string()]
    );
    scanner.clean_cache();
    assert!(scanner.scanned_component_ids().is_empty());
    assert!(scanner.spring_node_id_holder().get_node_id_set().is_empty());
}

/// 验证扫描工厂顺序、刷新作用域节点名和脚本代理均执行真实 Java 等价逻辑。
///
/// 对应 Java: `LiteflowScannerProcessStepFactory`、`SpringNodeIdHolder`、
/// `ScriptBeanProcess` 与 `ScriptMethodBeanProcess`。
#[test]
fn scanner_process_pipeline_preserves_priority_scope_and_script_registration() {
    let factory = LiteflowScannerProcessStepFactory::new();
    let priorities: Vec<_> = factory
        .get_process_steps()
        .iter()
        .map(|step| step.step_type().priority())
        .collect();
    assert_eq!(priorities, vec![1, 2, 3, 4, 5, 7]);
    assert_eq!(
        LiteflowScannerProcessStepEnum::DataBaseConnectBean.priority(),
        6
    );

    let counts = Arc::new(ManagedComponentCounts::default());
    let managed_component: Arc<dyn NodeComponent> = Arc::new(ManagedTestComponent {
        fail: false,
        name: "刷新作用域组件",
        counts,
    });
    let script_bean_proxy = ScriptBeanProxy::new(
        "vernal_process_script_bean",
        &[],
        &[],
        [ScriptMethodProxy::new(
            "sum",
            Arc::new(|arguments| {
                let sum = arguments.iter().filter_map(Value::as_i64).sum::<i64>();
                Ok(json!(sum))
            }),
        )],
    );
    let script_method_proxy = ScriptBeanProxy::new(
        "vernal_process_script_method",
        &[],
        &[],
        [ScriptMethodProxy::new(
            "echo",
            Arc::new(|arguments| Ok(arguments.first().cloned().unwrap_or(Value::Null))),
        )],
    );
    let scanner = VernalComponentScanner::with_config(
        &LiteflowConfig {
            print_banner: false,
            ..LiteflowConfig::default()
        },
        vec![
            LiteflowComponentRegistration::managed("scopedTarget.refresh_node", managed_component)
                .with_refresh_scope(),
            LiteflowComponentRegistration::script_bean("scriptBeanSource", script_bean_proxy),
            LiteflowComponentRegistration::script_methods(
                "scriptMethodSource",
                vec![script_method_proxy],
            ),
        ],
    );
    let flow_bus = liteflow_core::FlowBus::new();

    let managed_registrations = scanner.scan(&flow_bus).unwrap();
    assert_eq!(managed_registrations.len(), 1);
    assert_eq!(managed_registrations[0].component_id(), "refresh_node");
    assert_eq!(
        scanner.spring_node_id_holder().get_node_id_set(),
        vec!["refresh_node".to_string()]
    );
    assert_eq!(
        ScriptBeanManager::invoke("vernal_process_script_bean", "sum", &[json!(2), json!(3)])
            .unwrap(),
        json!(5)
    );
    assert_eq!(
        ScriptBeanManager::invoke("vernal_process_script_method", "echo", &[json!("真实调用")])
            .unwrap(),
        json!("真实调用")
    );
    ScriptBeanManager::remove_script_bean("vernal_process_script_bean");
    ScriptBeanManager::remove_script_bean("vernal_process_script_method");
}

/// 验证 Vernal 模块注册的 ContextAware 与容器解析到同一个对象，并暴露同一
/// `FlowBus`、`LiteflowRuntime` 和配置实例。
///
/// 对应 Java: `SpringAware#setApplicationContext` 与 `LiteflowSpiInit` 的装配闭环。
#[tokio::test]
async fn vernal_module_registers_real_context_aware_beans() {
    let module = LiteflowVernalModule::new(LiteflowConfig::default());
    let mut builder = VernalApplicationBuilder::current().unwrap();
    builder.register_module(module).unwrap();
    let context = builder.launch().await.unwrap();
    let aware: Arc<VernalAware> = context.container().resolve().unwrap();
    let runtime: Arc<LiteflowRuntime> = context.container().resolve().unwrap();
    let flow_bus: Arc<liteflow_core::FlowBus> = context.container().resolve().unwrap();
    let spi_init: Arc<LiteflowSpiInit> = context.container().resolve().unwrap();
    let component_scanner: Arc<VernalComponentScanner> = context.container().resolve().unwrap();
    let process_step_factory: Arc<LiteflowScannerProcessStepFactory> =
        context.container().resolve().unwrap();
    let spring_node_id_holder: Arc<SpringNodeIdHolder> = context.container().resolve().unwrap();
    let spring_cmp_around_aspect_holder: Arc<SpringCmpAroundAspectHolder> =
        context.container().resolve().unwrap();
    let decl_bean_definition: Arc<VernalDeclBeanDefinition> =
        context.container().resolve().unwrap();

    let aware_runtime = aware
        .get_bean("liteflowRuntime")
        .unwrap()
        .downcast::<LiteflowRuntime>()
        .unwrap();
    let aware_flow_bus = aware
        .get_bean("flowBus")
        .unwrap()
        .downcast::<liteflow_core::FlowBus>()
        .unwrap();
    let aware_config = aware
        .get_bean("liteflowConfig")
        .unwrap()
        .downcast::<LiteflowConfig>()
        .unwrap();

    assert!(Arc::ptr_eq(&aware_runtime, &runtime));
    assert!(Arc::ptr_eq(&aware_flow_bus, &flow_bus));
    assert_eq!(*aware_config, LiteflowConfig::default());
    assert_eq!(ContextAwareHolder::load_context_aware().priority(), 1);
    assert_eq!(
        CmpAroundAspectHolder::load_cmp_around_aspect().priority(),
        1
    );
    assert!(spi_init.is_initialized());
    assert!(decl_bean_definition.is_bean_factory_post_processed());
    assert!(component_scanner.scanned_component_ids().is_empty());
    assert_eq!(process_step_factory.get_process_steps().len(), 6);
    assert!(spring_node_id_holder.get_node_id_set().is_empty());
    assert!(spring_cmp_around_aspect_holder.get_instance().is_none());
    assert!(Arc::ptr_eq(
        &aware
            .get_bean("liteflowSpiInit")
            .unwrap()
            .downcast::<LiteflowSpiInit>()
            .unwrap(),
        &spi_init
    ));

    context.close().await.unwrap();
}

/// 验证 Vernal 容器切面和托管组件初始化 SPI 进入真实执行链，并保持 Java
/// before/success/error/after 调用次数。
///
/// 对应 Java: `SpringCmpAroundAspect`、`SpringContextCmpInit#initCmp`、
/// `NodeCmpBeanProcess` 与 `CmpAroundAspectBeanProcess`。
#[tokio::test]
async fn vernal_module_initializes_managed_nodes_and_delegates_global_aspect() {
    let ok_counts = Arc::new(ManagedComponentCounts::default());
    let error_counts = Arc::new(ManagedComponentCounts::default());
    let aspect_counts = Arc::new(ManagedAspectCounts::default());
    let ok_component: Arc<dyn NodeComponent> = Arc::new(ManagedTestComponent {
        fail: false,
        name: "Vernal 成功组件",
        counts: Arc::clone(&ok_counts),
    });
    let error_component: Arc<dyn NodeComponent> = Arc::new(ManagedTestComponent {
        fail: true,
        name: "Vernal 失败组件",
        counts: Arc::clone(&error_counts),
    });
    let aspect: Arc<dyn ICmpAroundAspect> = Arc::new(ManagedTestAspect {
        counts: Arc::clone(&aspect_counts),
    });
    let module = LiteflowVernalModule::new(
        LiteflowConfig::new().with_inline_rule(LiteflowRuleFormat::Json, MANAGED_COMPONENT_RULE),
    )
    .with_components([
        LiteflowComponentRegistration::managed("managed_ok", ok_component),
        LiteflowComponentRegistration::managed("managed_error", error_component),
    ])
    .with_cmp_around_aspect(aspect);
    let mut builder = VernalApplicationBuilder::current().unwrap();
    builder.register_module(module).unwrap();
    let context = builder.launch().await.unwrap();
    let runtime: Arc<LiteflowRuntime> = context.container().resolve().unwrap();
    let cmp_around_aspect: Arc<VernalCmpAroundAspect> = context.container().resolve().unwrap();
    let context_cmp_init: Arc<VernalContextCmpInit> = context.container().resolve().unwrap();
    let component_support: Arc<VernalLiteflowComponentSupport> =
        context.container().resolve().unwrap();

    assert_eq!(cmp_around_aspect.priority(), 1);
    assert_eq!(context_cmp_init.priority(), 1);
    assert_eq!(component_support.priority(), 1);
    assert_eq!(context_cmp_init.managed_node_count(), 2);
    assert!(runtime.flow_bus().contains_node("managed_ok"));
    assert!(runtime.flow_bus().contains_node("managed_error"));
    let node_map = runtime.flow_bus().get_node_map();
    assert_eq!(node_map["managed_ok"].name(), "Vernal 成功组件");
    assert_eq!(node_map["managed_error"].name(), "Vernal 失败组件");

    let response = runtime
        .try_execute("managed_component_chain", Value::Null)
        .await
        .unwrap();
    assert!(!response.is_success());
    assert_eq!(ok_counts.process.load(Ordering::SeqCst), 1);
    assert_eq!(error_counts.process.load(Ordering::SeqCst), 1);
    assert_eq!(aspect_counts.before.load(Ordering::SeqCst), 2);
    assert_eq!(aspect_counts.success.load(Ordering::SeqCst), 1);
    assert_eq!(aspect_counts.error.load(Ordering::SeqCst), 1);
    assert_eq!(aspect_counts.after.load(Ordering::SeqCst), 2);

    context.close().await.unwrap();
    ContextCmpInitHolder::clean();
}

/// 验证 Vernal 声明式解析器执行主方法校验、显式身份覆盖和空 ID 过滤。
///
/// 对应 Java:
/// `SpringDeclComponentParser#parseDeclBean(Class,String,String)`。
#[test]
fn vernal_decl_component_parser_preserves_spring_definition_rules() {
    let parser = VernalDeclComponentParser::new();
    let parsed = parser
        .parse_decl_bean_with_identity(
            declarative_component(
                "source_id",
                vec![declarative_method(LiteFlowMethodEnum::Process)],
            ),
            "override_id",
            "覆盖名称",
        )
        .unwrap();

    assert_eq!(parser.priority(), 1);
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].node_id(), "override_id");
    assert_eq!(parsed[0].node_name(), "覆盖名称");
    assert_eq!(parsed[0].node_type(), NodeTypeEnum::Common);

    let empty = parser
        .parse_decl_bean(declarative_component(
            "   ",
            vec![declarative_method(LiteFlowMethodEnum::Process)],
        ))
        .unwrap();
    assert!(empty.is_empty());

    let error = match parser.parse_decl_bean(declarative_component(
        "missing_process",
        vec![declarative_method(LiteFlowMethodEnum::BeforeProcess)],
    )) {
        Ok(_) => panic!("missing process method must be rejected"),
        Err(error) => error,
    };
    assert!(matches!(error, LiteflowError::CmpDefinition(_)));
    assert_eq!(
        error.to_string(),
        "Component [missing_process] does not define the process method"
    );
}

/// 验证声明式包装对象同时进入 Vernal 命名容器、优先级为 1 的解析器和真实
/// FlowBus 执行链，且三处共享同一个原始业务对象。
///
/// 对应 Java: `DeclBeanDefinition#registerNewBeanDefinition`、
/// `SpringDeclComponentParser#parseDeclBean`。
#[tokio::test]
async fn vernal_module_registers_and_executes_declarative_component() {
    let declaration = declarative_component(
        "vernal_decl",
        vec![declarative_method(LiteFlowMethodEnum::Process)],
    );
    let raw_bean = Arc::clone(declaration.raw_bean());
    let module = LiteflowVernalModule::new(
        LiteflowConfig::new()
            .with_inline_rule(LiteflowRuleFormat::Json, DECLARATIVE_COMPONENT_RULE),
    )
    .with_component(LiteflowComponentRegistration::declarative(declaration));
    let mut builder = VernalApplicationBuilder::current().unwrap();
    builder.register_module(module).unwrap();
    let context = builder.launch().await.unwrap();
    let runtime: Arc<LiteflowRuntime> = context.container().resolve().unwrap();
    let aware: Arc<VernalAware> = context.container().resolve().unwrap();
    let parser: Arc<VernalDeclComponentParser> = context.container().resolve().unwrap();
    let registered = aware
        .get_bean("vernal_decl")
        .unwrap()
        .downcast::<DeclWarpBean>()
        .unwrap();

    assert_eq!(parser.priority(), 1);
    assert!(Arc::ptr_eq(registered.raw_bean(), &raw_bean));
    let response = runtime
        .try_execute("declarative_component_chain", Value::Null)
        .await
        .unwrap();
    assert!(response.is_success(), "{}", response.message);
    assert_eq!(response.data("declarative_result"), Some(json!("vernal")));
    assert_eq!(response.steps[0].node_name, "Vernal 声明式组件");

    context.close().await.unwrap();
}

/// 验证 Vernal 对 Spring `classpath*:` 的跨资源匹配、稳定读取与绝对路径转换。
///
/// 对应 Java: `SpringPathContentParser#parseContent` 与
/// `SpringPathContentParser#getFileAbsolutePath`。
#[test]
fn vernal_path_content_parser_resolves_all_classpath_resources() {
    let parser = VernalPathContentParser::new();
    let pattern = vec!["classpath*:path_content_parser/multi/*.json".to_string()];

    let contents = parser.parse_content(&pattern).unwrap();
    assert_eq!(contents.len(), 2);
    assert!(contents[0].contains("classpath_first"));
    assert!(contents[1].contains("classpath_second"));

    let absolute_paths = parser.get_file_absolute_path(&pattern).unwrap();
    assert_eq!(absolute_paths.len(), 2);
    assert!(absolute_paths[0].ends_with("multi/first.json"));
    assert!(absolute_paths[1].ends_with("multi/second.json"));
    assert_eq!(parser.priority(), 1);
}

/// 验证 `classpath:` 与裸相对路径都只解析首个可用资源，并保留 Spring 对
/// 未匹配模式返回空集合的行为。
///
/// 对应 Java: `SpringPathContentParser#getResources`。
#[test]
fn vernal_path_content_parser_resolves_single_and_missing_resources() {
    let parser = VernalPathContentParser::new();
    let classpath = parser
        .parse_content(&["classpath:path_content_parser/multi/first.json".to_string()])
        .unwrap();
    let bare = parser
        .parse_content(&["path_content_parser/multi/first.json".to_string()])
        .unwrap();
    let missing = parser
        .parse_content(&["classpath*:path_content_parser/missing/*.json".to_string()])
        .unwrap();

    assert_eq!(classpath, bare);
    assert_eq!(classpath.len(), 1);
    assert!(classpath[0].contains("classpath_first"));
    assert!(missing.is_empty());
    assert!(
        parser
            .parse_content(&[])
            .unwrap_err()
            .to_string()
            .contains("rule source must not be null")
    );
}

/// 验证 Spring 路径解析器不允许同一规则源混用不同配置格式。
///
/// 对应 Java: `SpringPathContentParser#verifyFileExtName`。
#[test]
fn vernal_path_content_parser_rejects_mixed_extensions() {
    let error = VernalPathContentParser::new()
        .parse_content(&["classpath*:path_content_parser/mixed/*".to_string()])
        .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("config error,please use the same type of configuration")
    );
}

/// 验证绝对文件与 `file:` 资源仍按 Spring 本地文件语义解析。
///
/// 对应 Java: `SpringPathContentParser#getResources` 的绝对路径分支。
#[test]
fn vernal_path_content_parser_preserves_absolute_file_resources() {
    let absolute = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/resources/path_content_parser/multi/first.json")
        .canonicalize()
        .unwrap();
    let parser = VernalPathContentParser::new();

    let direct = parser
        .parse_content(&[absolute.to_string_lossy().into_owned()])
        .unwrap();
    let file_url = parser
        .parse_content(&[format!("file:{}", absolute.display())])
        .unwrap();

    assert_eq!(direct, file_url);
    assert!(direct[0].contains("classpath_first"));
}

/// 验证 Vernal 模块会在规则初始化前注册容器路径 SPI，并从两个 classpath
/// 资源构建真实 Chain。
///
/// 对应 Java: `LiteflowSpiInit#afterSingletonsInstantiated` 与
/// `SpringPathContentParser#parseContent` 的容器执行闭环。
#[tokio::test]
async fn vernal_module_builds_all_classpath_pattern_rules() {
    let registration = LiteflowComponentRegistration::new("vernal_component", |flow_bus| {
        flow_bus.register("vernal_component", cmp(|_| async { Ok(Value::Null) }));
        Ok(())
    });
    let module = LiteflowVernalModule::new(LiteflowConfig::new().with_rule_source(
        LiteflowRuleFormat::Json,
        "classpath*:path_content_parser/multi/*.json",
    ))
    .with_component(registration);
    let mut builder = VernalApplicationBuilder::current().unwrap();
    builder.register_module(module).unwrap();
    let context = builder.launch().await.unwrap();
    let runtime: Arc<LiteflowRuntime> = context.container().resolve().unwrap();

    assert_eq!(
        PathContentParserHolder::load_path_content_parser().priority(),
        1
    );
    for chain_id in ["classpath_first", "classpath_second"] {
        assert!(runtime.flow_bus().contains_chain(chain_id));
        let response = runtime.try_execute(chain_id, Value::Null).await.unwrap();
        assert!(response.is_success(), "{:?}", response.cause);
    }

    context.close().await.unwrap();
}

/// 对应 Spring `@ConfigurationProperties`：验证 serde camelCase 和枚举值。
#[test]
fn liteflow_config_deserializes_spring_style_properties() {
    let config: LiteflowConfig = serde_json::from_value(json!({
        "enable": true,
        "inlineRule": INLINE_RULE,
        "ruleFormat": "json",
        "parseMode": "PARSE_ALL_ON_START",
        "chainCacheEnabled": true,
        "chainCacheCapacity": 11,
        "printBanner": false,
        "monitorEnableLog": true,
        "queueLimit": 17,
        "delay": 23,
        "period": 29,
        "globalThreadPoolExecutorClass": "test.GlobalExecutor",
        "globalThreadPoolSize": 7,
        "globalThreadPoolQueueSize": 19,
        "mainExecutorClass": "test.MainExecutor",
        "mainExecutorWorks": 3,
        "whenThreadPoolIsolate": true,
        "enableVirtualThread": false
    }))
    .unwrap();

    assert!(config.enable);
    assert_eq!(config.inline_rule.as_deref(), Some(INLINE_RULE));
    assert!(config.monitor_enable_log);
    assert!(config.is_chain_cache_enabled());
    assert!(!config.print_banner);
    assert_eq!(config.chain_cache_capacity(), 11);
    assert_eq!(config.queue_limit(), 17);
    assert_eq!(config.delay(), 23);
    assert_eq!(config.period(), 29);
    assert_eq!(
        config.global_thread_pool_executor_class(),
        "test.GlobalExecutor"
    );
    assert_eq!(config.global_thread_pool_size(), 7);
    assert_eq!(config.global_thread_pool_queue_size(), 19);
    assert_eq!(config.main_executor_class(), "test.MainExecutor");
    assert_eq!(config.main_executor_works(), 3);
    assert!(config.is_when_thread_pool_isolate());
    assert!(!config.is_enable_virtual_thread());
    let core_config = config.to_core_config();
    assert!(core_config.is_chain_cache_enabled());
    assert_eq!(core_config.chain_cache_capacity(), 11);
    assert!(!core_config.get_print_banner());
}

/// 验证 Spring Boot 两个属性对象由 serde 绑定，并由独立自动配置逐字段合并。
///
/// 对应 Java: `LiteflowProperty`、`LiteflowMonitorProperty` 与
/// `LiteflowPropertyAutoConfiguration#liteflowConfig`。
#[test]
fn springboot_properties_merge_into_real_core_configuration() {
    let property: LiteflowProperty = serde_json::from_value(json!({
        "enable": true,
        "ruleSource": "classpath*:rules/*.xml",
        "ruleSourceExtData": "tenant=alpha",
        "ruleSourceExtDataMap": {"namespace": "prod"},
        "slotSize": 2048,
        "mainExecutorWorks": 9,
        "whenMaxWaitTime": 7,
        "whenMaxWaitTimeUnit": "SECONDS",
        "whenThreadPoolIsolate": true,
        "parseMode": "PARSE_ONE_ON_FIRST_EXEC",
        "supportMultipleType": true,
        "printBanner": false,
        "printExecutionLog": false,
        "enableMonitorFile": true,
        "fallbackCmpEnable": true,
        "fastLoad": true,
        "checkNodeExists": false,
        "scriptSetting": {"python": "-B"},
        "globalThreadPoolSize": 11,
        "globalThreadPoolQueueSize": 23,
        "enableNodeInstanceId": true,
        "enableVirtualThread": false,
        "chainCache": {"enabled": true, "capacity": 31}
    }))
    .unwrap();
    let monitor_property: LiteflowMonitorProperty = serde_json::from_value(json!({
        "enableLog": true,
        "queueLimit": 17,
        "delay": 19,
        "period": 29
    }))
    .unwrap();

    let config =
        LiteflowPropertyAutoConfiguration::new().liteflow_config(&property, &monitor_property);
    let core = config.to_core_config();

    assert_eq!(core.get_rule_source(), Some("classpath*:rules/*.xml"));
    assert_eq!(core.get_rule_source_ext_data(), Some("tenant=alpha"));
    assert_eq!(core.get_rule_source_ext_data_map()["namespace"], "prod");
    assert_eq!(core.get_slot_size(), 2048);
    assert_eq!(core.get_main_executor_works(), 9);
    assert_eq!(core.get_when_max_wait_time(), 7);
    assert_eq!(
        core.get_when_max_wait_time_unit(),
        liteflow_core::property::TimeUnit::Seconds
    );
    assert!(core.is_support_multiple_type());
    assert!(core.get_enable_monitor_file());
    assert!(core.get_fallback_cmp_enable());
    assert!(core.get_fast_load());
    assert_eq!(core.get_script_setting()["python"], "-B");
    assert_eq!(core.get_global_thread_pool_size(), 11);
    assert_eq!(core.get_global_thread_pool_queue_size(), 23);
    assert!(core.get_enable_node_instance_id());
    assert!(core.get_chain_cache_enabled());
    assert_eq!(core.get_chain_cache_capacity(), 31);
    assert!(!core.get_enable_virtual_thread());
    assert!(core.get_enable_log());
    assert_eq!(core.get_queue_limit(), 17);
    assert_eq!(core.get_delay(), 19);
    assert_eq!(core.get_period(), 29);
    assert!(!config.check_node_exists);
}

/// 验证主自动配置和执行器初始化对象进入真实 Vernal 启动/执行链。
///
/// 对应 Java: `LiteflowMainAutoConfiguration` 与
/// `LiteflowExecutorInit#afterSingletonsInstantiated`。
#[tokio::test]
async fn springboot_main_auto_configuration_eagerly_initializes_executor() {
    let auto_configuration = LiteflowMainAutoConfiguration::new(
        LiteflowConfig::new().with_inline_rule(LiteflowRuleFormat::Json, INLINE_RULE),
    )
    .with_component(LiteflowComponentRegistration::new(
        "vernal_component",
        |flow_bus| {
            flow_bus.register(
                "vernal_component",
                cmp(|context| async move {
                    context.set_data("springboot_auto_configured", json!(true));
                    Ok(Value::Null)
                }),
            );
            Ok(())
        },
    ));
    assert!(auto_configuration.is_enabled());

    let mut builder = VernalApplicationBuilder::current().unwrap();
    builder.register_module(auto_configuration).unwrap();
    let context = builder.launch().await.unwrap();
    let executor_init: Arc<LiteflowExecutorInit> = context.container().resolve().unwrap();
    let runtime: Arc<LiteflowRuntime> = context.container().resolve().unwrap();

    assert!(executor_init.is_initialized());
    assert!(runtime.flow_bus().contains_chain("vernal_chain"));
    let response = runtime
        .try_execute("vernal_chain", Value::Null)
        .await
        .unwrap();
    assert!(response.is_success(), "{}", response.message);
    assert_eq!(
        response.data("springboot_auto_configured"),
        Some(json!(true))
    );

    context.close().await.unwrap();
}

/// 验证 Spring Boot 4 使用独立属性对象完成 serde 绑定与字段合并。
///
/// 对应 Java: `com.yomahub.liteflow.springboot4.LiteflowProperty`、
/// `LiteflowMonitorProperty` 与
/// `LiteflowPropertyAutoConfiguration#liteflowConfig`。
#[test]
fn springboot4_properties_are_real_independent_objects() {
    use liteflow_vernal::springboot4::config::LiteflowPropertyAutoConfiguration;
    use liteflow_vernal::springboot4::{LiteflowMonitorProperty, LiteflowProperty};

    let property: LiteflowProperty = serde_json::from_value(json!({
        "ruleSource": "classpath:boot4.json",
        "slotSize": 4096,
        "parseMode": "PARSE_ONE_ON_FIRST_EXEC",
        "chainCache": {"enabled": true, "capacity": 47},
        "globalThreadPoolSize": 13,
        "enableVirtualThread": false
    }))
    .unwrap();
    let monitor_property: LiteflowMonitorProperty = serde_json::from_value(json!({
        "enableLog": true,
        "queueLimit": 37,
        "delay": 41,
        "period": 43
    }))
    .unwrap();

    let config =
        LiteflowPropertyAutoConfiguration::new().liteflow_config(&property, &monitor_property);
    let core = config.to_core_config();

    assert_eq!(core.get_rule_source(), Some("classpath:boot4.json"));
    assert_eq!(core.get_slot_size(), 4096);
    assert_eq!(
        core.get_parse_mode(),
        liteflow_core::enums::ParseModeEnum::ParseOneOnFirstExec
    );
    assert!(core.get_chain_cache_enabled());
    assert_eq!(core.get_chain_cache_capacity(), 47);
    assert_eq!(core.get_global_thread_pool_size(), 13);
    assert!(!core.get_enable_virtual_thread());
    assert!(core.get_enable_log());
    assert_eq!(core.get_queue_limit(), 37);
    assert_eq!(core.get_delay(), 41);
    assert_eq!(core.get_period(), 43);
}

/// 验证 Solon 首批基础对象具有真实属性、路径和上下文附件行为。
///
/// 对应 Java: `LiteflowMonitorProperty`、`PathsUtils`、`ResourceUtils` 与
/// `SolonNodeIdHolder`。
#[test]
fn solon_foundation_objects_preserve_configuration_and_context_isolation() {
    use liteflow_vernal::process::holder::SolonNodeIdHolder;
    use liteflow_vernal::solon::config::{LiteflowMonitorProperty, PathsUtils};
    use liteflow_vernal::spi::solon::ResourceUtils;

    let mut monitor_property: LiteflowMonitorProperty = serde_json::from_value(json!({
        "enableLog": true,
        "queueLimit": 17,
        "delay": 19,
        "period": 23
    }))
    .unwrap();
    assert!(monitor_property.is_enable_log());
    assert_eq!(monitor_property.get_queue_limit(), 17);
    assert_eq!(monitor_property.get_delay(), 19);
    assert_eq!(monitor_property.get_period(), 23);
    monitor_property.set_queue_limit(29);
    assert_eq!(monitor_property.get_queue_limit(), 29);

    let pattern = format!(
        "{}/tests/resources/path_content_parser/multi/*.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let paths = PathsUtils::resolve_paths(&pattern);
    assert_eq!(paths.len(), 2);
    assert!(paths.iter().all(|path| path.ends_with(".json")));
    assert_eq!(ResourceUtils::CLASSPATH_URL_PREFIX, "classpath:");
    assert_eq!(ResourceUtils::CLASSPATH_ALL_URL_PREFIX, "classpath*:");

    let first_context = VernalAware::new();
    let first_holder = SolonNodeIdHolder::of(&first_context);
    first_holder.add("node_a");
    first_holder.add("node_a");
    assert!(Arc::ptr_eq(
        &first_holder,
        &SolonNodeIdHolder::of(&first_context)
    ));
    assert_eq!(
        first_holder.get_node_id_set(),
        ["node_a".to_string()].into()
    );

    let second_context = VernalAware::new();
    let second_holder = SolonNodeIdHolder::of(&second_context);
    assert!(!Arc::ptr_eq(&first_holder, &second_holder));
    assert!(second_holder.get_node_id_set().is_empty());
}

/// 验证 Solon 主属性完整绑定、Java 默认回退与自动配置逐字段合并。
///
/// 对应 Java: `LiteflowProperty` 与
/// `LiteflowAutoConfiguration#liteflowConfig/monitorBus`。
#[test]
fn solon_property_and_auto_configuration_preserve_java_mapping() {
    use liteflow_vernal::solon::config::{
        LiteflowAutoConfiguration, LiteflowMonitorProperty, LiteflowProperty,
    };

    let pattern = format!(
        "{}/tests/resources/path_content_parser/multi/*.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut property: LiteflowProperty = serde_json::from_value(json!({
        "ruleSource": pattern,
        "ruleSourceExtData": "tenant=solon",
        "ruleSourceExtDataMap": {"namespace": "prod"},
        "slotSize": 2048,
        "mainExecutorWorks": 7,
        "mainExecutorClass": "custom.MainExecutor",
        "threadExecutorClass": "legacy.WhenExecutor",
        "whenMaxWaitSeconds": 9,
        "whenMaxWorkers": 11,
        "whenQueueLimit": 13,
        "parseMode": "PARSE_ONE_ON_FIRST_EXEC",
        "supportMultipleType": true,
        "retryCount": 2,
        "printBanner": false,
        "nodeExecutorClass": "custom.NodeExecutor",
        "requestIdGeneratorClass": "custom.RequestIdGenerator",
        "printExecutionLog": false,
        "parallelLoopExecutorClass": "legacy.LoopExecutor",
        "parallelMaxWorkers": 17,
        "parallelQueueLimit": 19,
        "fallbackCmpEnable": true,
        "globalThreadPoolExecutorClass": " ",
        "globalThreadPoolSize": null,
        "globalThreadPoolQueueSize": null,
        "whenThreadPoolIsolate": null,
        "enableNodeInstanceId": true,
        "chainCache": {"enabled": true, "capacity": 31}
    }))
    .unwrap();

    let resolved_paths = property
        .get_rule_source()
        .unwrap()
        .split(',')
        .collect::<Vec<_>>();
    assert_eq!(resolved_paths.len(), 2);
    assert_eq!(
        property.get_thread_executor_class(),
        Some("legacy.WhenExecutor")
    );
    assert_eq!(property.get_when_max_workers(), 11);
    assert_eq!(property.get_when_queue_limit(), 13);
    assert_eq!(property.get_parallel_max_workers(), Some(17));
    assert_eq!(property.get_parallel_queue_limit(), Some(19));
    assert_eq!(
        property.get_global_thread_pool_executor_class(),
        "com.yomahub.liteflow.thread.LiteFlowDefaultGlobalExecutorBuilder"
    );
    assert_eq!(property.get_global_thread_pool_size(), 16);
    assert_eq!(property.get_global_thread_pool_queue_size(), 512);
    assert!(!property.get_when_thread_pool_isolate());

    property.set_parallel_max_workers(Some(23));
    assert_eq!(property.get_parallel_max_workers(), Some(23));

    let monitor_property: LiteflowMonitorProperty = serde_json::from_value(json!({
        "enableLog": true,
        "queueLimit": 37,
        "delay": 41,
        "period": 43
    }))
    .unwrap();
    let auto_configuration = LiteflowAutoConfiguration::new(true);
    let config = auto_configuration.liteflow_config(&property, &monitor_property);
    let core = config.to_core_config();

    assert_eq!(core.get_slot_size(), 2048);
    assert_eq!(core.get_main_executor_works(), 7);
    #[allow(deprecated)]
    let when_max_wait_seconds = core.get_when_max_wait_seconds();
    #[allow(deprecated)]
    let retry_count = core.get_retry_count();
    assert_eq!(when_max_wait_seconds, Some(9));
    assert!(core.is_support_multiple_type());
    assert_eq!(retry_count, 2);
    assert!(!core.get_print_banner());
    assert!(!core.get_print_execution_log());
    assert!(core.get_fallback_cmp_enable());
    assert_eq!(core.get_global_thread_pool_size(), 16);
    assert_eq!(core.get_global_thread_pool_queue_size(), 512);
    assert!(core.get_enable_node_instance_id());
    assert!(core.get_chain_cache_enabled());
    assert_eq!(core.get_chain_cache_capacity(), 31);
    assert!(core.get_enable_log());
    assert_eq!(core.get_queue_limit(), 37);

    let monitor_bus = auto_configuration.monitor_bus(&config).unwrap();
    assert_eq!(monitor_bus.get_liteflow_config().get_queue_limit(), 37);
    assert!(
        LiteflowAutoConfiguration::new(false)
            .monitor_bus(&config)
            .is_none()
    );
}

/// 验证 Solon `parseOnStart=false` 不在容器启动期解析规则，并且不会注册
/// Spring Boot 专属的执行器初始化对象。
///
/// 对应 Java: `LiteflowMainAutoConfiguration#flowExecutor`。
#[tokio::test]
async fn solon_main_auto_configuration_defers_rule_parsing() {
    use liteflow_vernal::solon::config::LiteflowMainAutoConfiguration;

    let invalid_rule = "{ invalid-json";
    let auto_configuration = LiteflowMainAutoConfiguration::new(
        LiteflowConfig::new().with_inline_rule(LiteflowRuleFormat::Json, invalid_rule),
    )
    .with_parse_on_start(false);
    assert!(!auto_configuration.is_parse_on_start());

    let mut builder = VernalApplicationBuilder::current().unwrap();
    builder.register_module(auto_configuration).unwrap();
    let context = builder.launch().await.unwrap();
    let runtime: Arc<LiteflowRuntime> = context.container().resolve().unwrap();
    assert!(
        context
            .container()
            .resolve::<LiteflowExecutorInit>()
            .is_err()
    );
    assert!(runtime.flow_bus().chain_ids().is_empty());
    assert!(
        runtime
            .try_execute("broken_chain", Value::Null)
            .await
            .is_err()
    );
    context.close().await.unwrap();
}

/// 验证 Solon 主自动配置只注册 Solon 专属 SPI，并通过 XPluginImpl 完成普通节点、
/// 声明式节点与全局切面的真实执行闭环。
///
/// 对应 Java: `XPluginImpl#start` 与 6 个 `spi.solon` 实现。
#[tokio::test]
async fn solon_plugin_registers_exclusive_spis_and_executes_components() {
    use liteflow_vernal::solon::config::LiteflowMainAutoConfiguration;
    use liteflow_vernal::solon::integration::XPluginImpl;
    use liteflow_vernal::spi::solon::{
        SolonCmpAroundAspect, SolonContextAware, SolonContextCmpInit, SolonDeclComponentParser,
        SolonLiteflowComponentSupport, SolonPathContentParser,
    };

    let rule = r#"{
      "flow": {
        "chain": [{
          "id": "solon_chain",
          "body": "THEN(solon_managed, solon_decl.process)"
        }]
      }
    }"#;
    let managed_counts = Arc::new(ManagedComponentCounts::default());
    let aspect_counts = Arc::new(ManagedAspectCounts::default());
    let managed_component: Arc<dyn NodeComponent> = Arc::new(ManagedTestComponent {
        fail: false,
        name: "Solon 托管组件",
        counts: Arc::clone(&managed_counts),
    });
    let aspect: Arc<dyn ICmpAroundAspect> = Arc::new(ManagedTestAspect {
        counts: Arc::clone(&aspect_counts),
    });
    let auto_configuration = LiteflowMainAutoConfiguration::new(
        LiteflowConfig::new().with_inline_rule(LiteflowRuleFormat::Json, rule),
    )
    .with_components([
        LiteflowComponentRegistration::managed("solon_managed", managed_component),
        LiteflowComponentRegistration::declarative(declarative_component(
            "solon_decl",
            vec![declarative_method(LiteFlowMethodEnum::Process)],
        )),
    ])
    .with_cmp_around_aspect(aspect);

    let mut builder = VernalApplicationBuilder::current().unwrap();
    builder.register_module(auto_configuration).unwrap();
    let context = builder.launch().await.unwrap();
    let runtime: Arc<LiteflowRuntime> = context.container().resolve().unwrap();
    let solon_context: Arc<SolonContextAware> = context.container().resolve().unwrap();
    let solon_aspect: Arc<SolonCmpAroundAspect> = context.container().resolve().unwrap();
    let solon_context_cmp_init: Arc<SolonContextCmpInit> = context.container().resolve().unwrap();
    let _: Arc<SolonDeclComponentParser> = context.container().resolve().unwrap();
    let component_support: Arc<SolonLiteflowComponentSupport> =
        context.container().resolve().unwrap();
    let _: Arc<SolonPathContentParser> = context.container().resolve().unwrap();
    let plugin: Arc<XPluginImpl> = context.container().resolve().unwrap();

    assert!(plugin.is_default_properties_loaded());
    assert!(plugin.is_started());
    assert!(solon_context.has_bean("liteflowProperty"));
    assert!(solon_context.has_bean("liteflowMonitorProperty"));
    assert!(solon_context.has_bean("liteflowAutoConfiguration"));
    assert_eq!(solon_context_cmp_init.managed_node_count(), 1);
    assert!(solon_aspect.get_cmp_around_aspect().is_some());
    assert!(
        context.container().resolve::<VernalAware>().is_err(),
        "Solon 模块不得通过 VernalAware 冒充 SolonContextAware"
    );
    assert_eq!(
        component_support.get_cmp_name(
            solon_context
                .get_node_component("solon_managed")
                .unwrap()
                .as_ref()
        ),
        Some("Solon 托管组件".to_string())
    );

    let response = runtime
        .try_execute("solon_chain", Value::Null)
        .await
        .unwrap();
    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(managed_counts.process.load(Ordering::SeqCst), 1);
    assert_eq!(response.data("declarative_result"), Some(json!("vernal")));
    assert!(aspect_counts.before.load(Ordering::SeqCst) >= 2);
    assert_eq!(
        aspect_counts.before.load(Ordering::SeqCst),
        aspect_counts.after.load(Ordering::SeqCst)
    );
    assert_eq!(
        aspect_counts.before.load(Ordering::SeqCst),
        aspect_counts.success.load(Ordering::SeqCst)
    );
    assert_eq!(aspect_counts.error.load(Ordering::SeqCst), 0);
    context.close().await.unwrap();
}

/// 验证 Solon 路径 SPI 的 classpath、绝对路径、多资源和混合扩展名约束。
///
/// 对应 Java: `SolonPathContentParser#parseContent/getFileAbsolutePath`。
#[test]
fn solon_path_content_parser_reads_real_resources() {
    use liteflow_vernal::spi::solon::SolonPathContentParser;

    let parser = SolonPathContentParser::new();
    let single = parser
        .parse_content(&["classpath:path_content_parser/multi/first.json".to_string()])
        .unwrap();
    assert_eq!(single.len(), 1);
    assert!(!single[0].trim().is_empty());

    let multiple = parser
        .parse_content(&["classpath*:path_content_parser/multi/*.json".to_string()])
        .unwrap();
    assert_eq!(multiple.len(), 2);
    let absolute_paths = parser
        .get_file_absolute_path(&[format!(
            "{}/tests/resources/path_content_parser/multi/*.json",
            env!("CARGO_MANIFEST_DIR")
        )])
        .unwrap();
    assert_eq!(absolute_paths.len(), 2);

    let mixed = parser.parse_content(&["classpath*:path_content_parser/mixed/*".to_string()]);
    assert!(mixed.is_err());
    assert!(parser.parse_content(&[]).is_err());
}

/// 验证关闭 LiteFlow 时 Solon 插件只加载默认配置，不创建运行时或配置 Bean。
///
/// 对应 Java: `XPluginImpl#start` 中 `liteflow.enable=false` 的提前返回。
#[tokio::test]
async fn solon_plugin_disabled_short_circuits_before_bean_creation() {
    use liteflow_vernal::solon::config::LiteflowMainAutoConfiguration;
    use liteflow_vernal::solon::integration::XPluginImpl;
    use liteflow_vernal::spi::solon::SolonContextAware;

    let mut config = LiteflowConfig::default();
    config.enable = false;
    let mut builder = VernalApplicationBuilder::current().unwrap();
    builder
        .register_module(LiteflowMainAutoConfiguration::new(config))
        .unwrap();
    let context = builder.launch().await.unwrap();
    let plugin: Arc<XPluginImpl> = context.container().resolve().unwrap();
    let solon_context: Arc<SolonContextAware> = context.container().resolve().unwrap();

    assert!(plugin.is_default_properties_loaded());
    assert!(!plugin.is_started());
    assert!(!solon_context.has_bean("liteflowProperty"));
    assert!(context.container().resolve::<LiteflowRuntime>().is_err());
    context.close().await.unwrap();
}

/// 验证 Boot 4 自动配置注册的是 Boot 4 初始化对象，而不是 Boot 3 同名类型。
///
/// 对应 Java:
/// `com.yomahub.liteflow.springboot4.config.LiteflowMainAutoConfiguration` 与
/// `com.yomahub.liteflow.springboot4.LiteflowExecutorInit`。
#[tokio::test]
async fn springboot4_main_auto_configuration_registers_boot4_lifecycle() {
    use liteflow_vernal::springboot4::LiteflowExecutorInit;
    use liteflow_vernal::springboot4::config::LiteflowMainAutoConfiguration;

    let auto_configuration = LiteflowMainAutoConfiguration::new(
        LiteflowConfig::new().with_inline_rule(LiteflowRuleFormat::Json, INLINE_RULE),
    )
    .with_component(LiteflowComponentRegistration::new(
        "vernal_component",
        |flow_bus| {
            flow_bus.register(
                "vernal_component",
                cmp(|context| async move {
                    context.set_data("springboot4_auto_configured", json!(true));
                    Ok(Value::Null)
                }),
            );
            Ok(())
        },
    ));

    let mut builder = VernalApplicationBuilder::current().unwrap();
    builder.register_module(auto_configuration).unwrap();
    let context = builder.launch().await.unwrap();
    let executor_init: Arc<LiteflowExecutorInit> = context.container().resolve().unwrap();
    let boot3_init = context
        .container()
        .resolve::<liteflow_vernal::LiteflowExecutorInit>();
    let runtime: Arc<LiteflowRuntime> = context.container().resolve().unwrap();

    assert!(executor_init.is_initialized());
    assert!(boot3_init.is_err(), "Boot 4 容器不应注册 Boot 3 初始化对象");
    let response = runtime
        .try_execute("vernal_chain", Value::Null)
        .await
        .unwrap();
    assert!(response.is_success(), "{}", response.message);
    assert_eq!(
        response.data("springboot4_auto_configured"),
        Some(json!(true))
    );

    context.close().await.unwrap();
}

/// 验证 Java MonitorBus 构造语义：配置容量、自动启动以及容器关闭时停止调度器。
#[tokio::test]
async fn vernal_lifecycle_starts_and_stops_configured_monitor_task() {
    let config = LiteflowConfig {
        monitor_enable_log: true,
        queue_limit: 3,
        delay: 1,
        period: 2,
        ..LiteflowConfig::default()
    };
    let module = LiteflowVernalModule::new(config);
    let mut builder = VernalApplicationBuilder::current().unwrap();
    builder.register_module(module).unwrap();
    let context = builder.launch().await.unwrap();
    let runtime: Arc<LiteflowRuntime> = context.container().resolve().unwrap();

    assert_eq!(runtime.flow_bus().monitor().queue_limit(), 3);
    assert!(runtime.is_monitor_task_running());
    tokio::time::sleep(std::time::Duration::from_millis(8)).await;
    assert!(runtime.is_monitor_task_running());

    context.close().await.unwrap();
    assert!(!runtime.is_monitor_task_running());
}

/// 验证 Java LiteflowConfigGetter 的 set/get/clean 回退契约。
#[test]
fn liteflow_config_getter_preserves_compatibility_contract() {
    let configured = LiteflowConfig {
        print_execution_log: false,
        monitor_enable_log: true,
        ..LiteflowConfig::default()
    };
    LiteflowConfigGetter::set_liteflow_config(configured.to_core_config());
    assert_eq!(LiteflowConfigGetter::get(), configured.to_core_config());

    LiteflowConfigGetter::clean();
    assert_eq!(
        LiteflowConfigGetter::get(),
        LiteflowConfig::default().to_core_config()
    );
}

/// 验证三种 Java ParseMode 枚举值均可从 Spring 风格配置反序列化。
#[test]
fn liteflow_config_deserializes_all_java_parse_modes() {
    for (source, expected) in [
        ("PARSE_ALL_ON_START", LiteflowParseMode::ParseAllOnStart),
        (
            "PARSE_ALL_ON_FIRST_EXEC",
            LiteflowParseMode::ParseAllOnFirstExec,
        ),
        (
            "PARSE_ONE_ON_FIRST_EXEC",
            LiteflowParseMode::ParseOneOnFirstExec,
        ),
    ] {
        let config: LiteflowConfig = serde_json::from_value(json!({"parseMode": source})).unwrap();
        assert_eq!(config.parse_mode, expected);
    }
}

/// 验证首次执行全解析：执行前无 Chain，并发首次请求后全部 Chain 与脚本均已物化。
#[tokio::test]
async fn parse_all_on_first_exec_is_concurrency_safe_and_builds_every_rule() {
    let runtime = Arc::new(LiteflowRuntime::new(
        liteflow_core::FlowBus::new(),
        LiteflowConfig {
            inline_rule: Some(LAZY_JSON_RULE.to_string()),
            parse_mode: LiteflowParseMode::ParseAllOnFirstExec,
            ..LiteflowConfig::default()
        },
    ));
    assert!(runtime.flow_bus().chain_ids().is_empty());

    let mut tasks = Vec::new();
    for _ in 0..8 {
        let runtime = Arc::clone(&runtime);
        tasks.push(tokio::spawn(async move {
            runtime
                .try_execute("root_chain", Value::Null)
                .await
                .unwrap()
        }));
    }
    for task in tasks {
        let response = task.await.unwrap();
        assert!(response.is_success(), "{:?}", response.cause);
    }

    for chain_id in ["root_chain", "child_chain", "unused_chain"] {
        assert!(runtime.flow_bus().contains_chain(chain_id));
    }
    for node_id in ["root_script", "child_script", "unused_script"] {
        assert!(runtime.flow_bus().contains_node(node_id));
    }
}

/// 验证首次执行单链解析在 JSON/XML/YAML 中只物化目标依赖闭包。
#[tokio::test]
async fn parse_one_on_first_exec_builds_only_target_dependency_closure() {
    for (format, source) in [
        (LiteflowRuleFormat::Json, LAZY_JSON_RULE),
        (LiteflowRuleFormat::Xml, LAZY_XML_RULE),
        (LiteflowRuleFormat::Yml, LAZY_YML_RULE),
    ] {
        let runtime = LiteflowRuntime::new(
            liteflow_core::FlowBus::new(),
            LiteflowConfig {
                inline_rule: Some(source.to_string()),
                rule_format: format,
                parse_mode: LiteflowParseMode::ParseOneOnFirstExec,
                ..LiteflowConfig::default()
            },
        );
        assert!(runtime.flow_bus().chain_ids().is_empty());
        assert!(!runtime.flow_bus().contains_node("root_script"));

        let response = runtime
            .try_execute("root_chain", Value::Null)
            .await
            .unwrap();
        assert!(response.is_success(), "{:?}", response.cause);
        assert!(runtime.flow_bus().contains_chain("root_chain"));
        assert!(runtime.flow_bus().contains_chain("child_chain"));
        assert!(runtime.flow_bus().contains_node("root_script"));
        assert!(runtime.flow_bus().contains_node("child_script"));
        assert!(!runtime.flow_bus().contains_chain("unused_chain"));
        assert!(!runtime.flow_bus().contains_node("unused_script"));
    }
}

/// 验证 Chain 缓存淘汰后由既有规则计划重新物化，而不是永久丢失 Chain。
///
/// `root_chain` 会嵌套执行 `child_chain`；Java `ChainCacheLifeCycle` 会把主链和
/// 子链都计入缓存，因此容量设置为 2，执行第三条 `unused_chain` 时才会淘汰
/// 最早访问的 `root_chain`。
///
/// 对应 Java `ChainCacheLifeCycle` + `PARSE_ONE_ON_FIRST_EXEC` 的完整执行闭环。
#[tokio::test]
async fn parse_one_chain_cache_evicts_and_rebuilds_compiled_chain() {
    let runtime = LiteflowRuntime::new(
        liteflow_core::FlowBus::new(),
        LiteflowConfig {
            inline_rule: Some(LAZY_JSON_RULE.to_string()),
            rule_format: LiteflowRuleFormat::Json,
            parse_mode: LiteflowParseMode::ParseOneOnFirstExec,
            chain_cache_enabled: true,
            chain_cache_capacity: 2,
            ..LiteflowConfig::default()
        },
    );

    let root = runtime
        .try_execute("root_chain", Value::Null)
        .await
        .unwrap();
    assert!(root.is_success(), "{:?}", root.cause);
    assert!(runtime.flow_bus().contains_chain("root_chain"));

    let unused = runtime
        .try_execute("unused_chain", Value::Null)
        .await
        .unwrap();
    assert!(unused.is_success(), "{:?}", unused.cause);
    assert!(
        !runtime.flow_bus().contains_chain("root_chain"),
        "容量为 2 时主链及子链已占满缓存，执行第三条链必须淘汰最早访问的主链"
    );
    assert!(runtime.flow_bus().contains_chain("unused_chain"));

    let rebuilt = runtime
        .try_execute("root_chain", Value::Null)
        .await
        .unwrap();
    assert!(rebuilt.is_success(), "{:?}", rebuilt.cause);
    assert!(runtime.flow_bus().contains_chain("root_chain"));
    assert!(
        !runtime.flow_bus().contains_chain("unused_chain"),
        "重新执行被淘汰链时应由规则计划重建，并淘汰上一条链"
    );
}

/// 验证 Java `FlowExecutor#initChainCache` 的容量校验不会被 Rust 默认值掩盖。
#[tokio::test]
async fn parse_one_chain_cache_rejects_zero_capacity() {
    let runtime = LiteflowRuntime::new(
        liteflow_core::FlowBus::new(),
        LiteflowConfig {
            inline_rule: Some(LAZY_JSON_RULE.to_string()),
            parse_mode: LiteflowParseMode::ParseOneOnFirstExec,
            chain_cache_enabled: true,
            chain_cache_capacity: 0,
            ..LiteflowConfig::default()
        },
    );

    let error = runtime
        .try_execute("root_chain", Value::Null)
        .await
        .expect_err("容量为 0 必须在规则执行前拒绝初始化");
    assert!(error.to_string().contains("greater than 0"));
    assert!(runtime.flow_bus().chain_ids().is_empty());
}

/// 验证文件规则源同样经过按链延迟计划，而不是回退为启动期全量构建。
#[tokio::test]
async fn parse_one_on_first_exec_supports_file_rule_source() {
    let file = std::env::temp_dir().join(format!(
        "liteflow-parse-one-{}-{}.json",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    std::fs::write(&file, LAZY_JSON_RULE).unwrap();
    let runtime = LiteflowRuntime::new(
        liteflow_core::FlowBus::new(),
        LiteflowConfig {
            rule_source: Some(file.to_string_lossy().into_owned()),
            parse_mode: LiteflowParseMode::ParseOneOnFirstExec,
            ..LiteflowConfig::default()
        },
    );
    // 对应 Java `LiteflowExecutorInit#afterSingletonsInstantiated` 的
    // `flowExecutor.init(true)` + `FlowBus.needInit()`：领取首次初始化门闩，
    // 使后续执行只走 vernal 的按链延迟计划，而不是被 FlowExecutor 内部的
    // needInit 兜底分支全量解析（该兜底只服务于无容器直接使用 core 的场景）。
    runtime.initialize_executor().unwrap();

    let response = runtime
        .try_execute("child_chain", Value::Null)
        .await
        .unwrap();
    let _ = std::fs::remove_file(file);

    assert!(response.is_success(), "{:?}", response.cause);
    assert!(runtime.flow_bus().contains_chain("child_chain"));
    assert!(runtime.flow_bus().contains_node("child_script"));
    assert!(!runtime.flow_bus().contains_chain("root_chain"));
    assert!(!runtime.flow_bus().contains_node("root_script"));
    assert!(!runtime.flow_bus().contains_node("unused_script"));
}

/// 验证 Vernal 启动阶段区分两种延迟模式：全量模式不读规则，单链模式只收集定义。
#[tokio::test]
async fn vernal_lifecycle_preserves_both_lazy_parse_mode_boundaries() {
    for parse_mode in [
        LiteflowParseMode::ParseAllOnFirstExec,
        LiteflowParseMode::ParseOneOnFirstExec,
    ] {
        let module = LiteflowVernalModule::new(LiteflowConfig {
            inline_rule: Some(LAZY_JSON_RULE.to_string()),
            parse_mode,
            ..LiteflowConfig::default()
        });
        let mut builder = VernalApplicationBuilder::current().unwrap();
        builder.register_module(module).unwrap();
        let context = builder.launch().await.unwrap();
        let runtime: Arc<LiteflowRuntime> = context.container().resolve().unwrap();

        assert!(runtime.flow_bus().chain_ids().is_empty());
        assert!(!runtime.flow_bus().contains_node("root_script"));
        let response = runtime
            .try_execute("root_chain", Value::Null)
            .await
            .unwrap();
        assert!(response.is_success(), "{:?}", response.cause);
        assert!(runtime.flow_bus().contains_chain("root_chain"));
        assert!(runtime.flow_bus().contains_chain("child_chain"));
        assert_eq!(
            runtime.flow_bus().contains_chain("unused_chain"),
            parse_mode == LiteflowParseMode::ParseAllOnFirstExec
        );
        assert_eq!(
            runtime.flow_bus().contains_node("unused_script"),
            parse_mode == LiteflowParseMode::ParseAllOnFirstExec
        );
        context.close().await.unwrap();
    }
}

/// 验证 ApplicationModule 原子装配、Lifecycle 规则初始化和同一容器解析。
#[tokio::test]
async fn vernal_module_launches_and_executes_registered_component() {
    let context = ready_context().await;
    let runtime: Arc<LiteflowRuntime> = context.container().resolve().unwrap();
    let flow_bus: Arc<liteflow_core::FlowBus> = context.container().resolve().unwrap();

    let response = runtime.execute("vernal_chain", json!({"value": 42})).await;

    assert!(response.is_success(), "{:?}", response.cause);
    assert_eq!(
        response.data("observed"),
        Some(json!(42)),
        "steps={:?}, chain_ids={:?}, node_ids={:?}",
        response
            .steps
            .iter()
            .map(|step| step.node_id.clone())
            .collect::<Vec<_>>(),
        flow_bus.chain_ids(),
        flow_bus.get_node_map().keys().cloned().collect::<Vec<_>>()
    );
    assert!(flow_bus.contains_node("vernal_component"));
    context.close().await.unwrap();
}

/// 验证两个 LiteFlow 运行时并发执行时，请求 Slot、Chain LRU 和组件数据不会串扰。
///
/// Java 单应用依赖进程级 Holder；Rust/Vernal 必须允许多个 ApplicationContext
/// 共存，因此把普通组件执行与独立 LRU 运行时反复交错，覆盖曾经只在默认并行
/// 测试调度下出现的“响应数据丢失 + LRU 淘汰错误”组合故障。
#[tokio::test]
async fn concurrent_runtimes_isolate_slot_data_and_chain_cache() {
    let context = ready_context().await;
    let component_runtime: Arc<LiteflowRuntime> = context.container().resolve().unwrap();

    for value in 0..128_u64 {
        let cache_runtime = LiteflowRuntime::new(
            liteflow_core::FlowBus::new(),
            LiteflowConfig {
                inline_rule: Some(LAZY_JSON_RULE.to_string()),
                rule_format: LiteflowRuleFormat::Json,
                parse_mode: LiteflowParseMode::ParseOneOnFirstExec,
                chain_cache_enabled: true,
                chain_cache_capacity: 2,
                ..LiteflowConfig::default()
            },
        );
        let component_execution =
            component_runtime.execute("vernal_chain", json!({"value": value}));
        let cache_execution = async {
            let root = cache_runtime
                .try_execute("root_chain", Value::Null)
                .await
                .unwrap();
            assert!(root.is_success(), "{:?}", root.cause);
            let unused = cache_runtime
                .try_execute("unused_chain", Value::Null)
                .await
                .unwrap();
            assert!(unused.is_success(), "{:?}", unused.cause);
            let rebuilt = cache_runtime
                .try_execute("root_chain", Value::Null)
                .await
                .unwrap();
            assert!(rebuilt.is_success(), "{:?}", rebuilt.cause);
            assert!(
                !cache_runtime.flow_bus().contains_chain("unused_chain"),
                "第 {value} 轮重新执行 root_chain 后必须淘汰上一条 unused_chain；chains={:?}，steps={:?}",
                cache_runtime.flow_bus().chain_ids(),
                rebuilt
                    .steps
                    .iter()
                    .map(|step| step.node_id.clone())
                    .collect::<Vec<_>>()
            );
        };

        let (component_response, ()) = tokio::join!(component_execution, cache_execution);
        assert!(
            component_response.is_success(),
            "{:?}",
            component_response.cause
        );
        assert_eq!(component_response.data("observed"), Some(json!(value)));
    }

    context.close().await.unwrap();
}

/// 验证真实 Axum Router、Vernal 请求作用域和 JSON 协议边界。
#[cfg(feature = "axum")]
#[tokio::test]
async fn axum_endpoint_resolves_runtime_from_vernal_context() {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use liteflow_vernal::LiteflowAxumRouter;
    use tower::ServiceExt;

    let context = ready_context().await;
    let app = LiteflowAxumRouter::with_context(Arc::clone(&context));
    let request = Request::builder()
        .method("POST")
        .uri("/liteflow/execute/vernal_chain")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"data": {"value": 7}, "requestId": "http-rid"}).to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: LiteflowExecuteResponse = serde_json::from_slice(&bytes).unwrap();
    assert!(body.success);
    assert_eq!(body.request_id, "http-rid");
    assert!(body.steps.contains("vernal_component"));
    context.close().await.unwrap();
}

/// 验证真实 Actix App、Vernal 中间件和组件提取器。
#[cfg(feature = "actix")]
#[actix_web::test]
async fn actix_endpoint_resolves_runtime_from_vernal_context() {
    use actix_web::{App, http::StatusCode, test};
    use liteflow_vernal::LiteflowActixService;

    let context = ready_context().await;
    let service = test::init_service(App::new().configure({
        let context = Arc::clone(&context);
        move |config| LiteflowActixService::configure(config, context)
    }))
    .await;
    let request = test::TestRequest::post()
        .uri("/liteflow/execute/vernal_chain")
        .set_json(json!({"data": {"value": 9}, "requestId": "actix-rid"}))
        .to_request();

    let response = test::call_service(&service, request).await;

    assert_eq!(response.status(), StatusCode::OK);
    let body: LiteflowExecuteResponse = test::read_body_json(response).await;
    assert!(body.success);
    assert_eq!(body.request_id, "actix-rid");
    context.close().await.unwrap();
}
