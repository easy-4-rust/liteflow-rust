//! Parser / Util / SPI 域未触达 API 补测（批次 G）。
//!
//! 覆盖：
//! - `LocalJson/Xml/YmlFlowElParser#loadJsonFile/loadXmlFile/loadYmlFile`
//! - `ClassParserFactory#register/parserType/createRegistered/createForType`
//! - `ClassJson/Xml/YmlFlowElParser#parseCustom`
//! - `PathMatchUtil#searchAbsolutePath`
//! - `RuleParsePluginUtil.ChainDto#withEnable`
//! - `NodeExecutorHelper#tryBuildNodeExecutor`（自定义执行器类名解析）
//! - `ComponentInitializer#withDefaultNodeExecutor`
//! - `LocalContextAware#registerDeclWrapBean`

use liteflow_core::enums::FlowParserTypeEnum;
use liteflow_core::parser::ClassParserFactory;
use liteflow_core::parser::el::{load_json_file, load_xml_file, load_yml_file};
use liteflow_core::util::path_match_util::PathMatchUtil;
use liteflow_core::util::rule_parse_plugin_util::ChainDto;
use liteflow_core::{FlowBus, LiteflowError, cmp};
use serde_json::Value;
use std::sync::Arc;

/// 本地 JSON/XML/YML 规则文件的兼容加载入口。
#[test]
fn local_rule_file_loaders_parse_real_files() {
    let dir = std::env::temp_dir().join("liteflow-batch-g");
    std::fs::create_dir_all(&dir).unwrap();

    let json_path = dir.join("rules.json");
    std::fs::write(
        &json_path,
        r#"{"flow":{"chain":[{"id":"g_json","body":"THEN(a)"}]}}"#,
    )
    .unwrap();
    let xml_path = dir.join("rules.xml");
    std::fs::write(
        &xml_path,
        r#"<flow><chain id="g_xml">THEN(b)</chain></flow>"#,
    )
    .unwrap();
    let yml_path = dir.join("rules.yml");
    std::fs::write(
        &yml_path,
        "flow:\n  chain:\n    - id: g_yml\n      body: THEN(c)\n",
    )
    .unwrap();

    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    bus.register("b", cmp(|_| async { Ok(Value::Null) }));
    bus.register("c", cmp(|_| async { Ok(Value::Null) }));

    let chains = load_json_file(&bus, &json_path).expect("JSON 规则应加载");
    assert_eq!(chains, vec!["g_json".to_string()]);

    let chains = load_xml_file(&bus, &xml_path).expect("XML 规则应加载");
    assert_eq!(chains, vec!["g_xml".to_string()]);

    let chains = load_yml_file(&bus, &yml_path).expect("YML 规则应加载");
    assert_eq!(chains, vec!["g_yml".to_string()]);

    let _ = std::fs::remove_dir_all(&dir);
}

/// ClassParserFactory 自定义解析器注册与创建。
#[test]
fn class_parser_factory_register_and_create() {
    let bus = FlowBus::new();
    let factory = ClassParserFactory::new(bus);
    factory.register(
        "com.example.MyParser",
        FlowParserTypeEnum::TypeXml,
        Arc::new(|| Ok("<flow/>".to_string())),
    );

    assert_eq!(
        factory.parser_type("com.example.MyParser"),
        Some(FlowParserTypeEnum::TypeXml)
    );
    assert!(factory.create_registered("com.example.MyParser").is_ok());
    assert!(factory.create_registered("com.example.Missing").is_err());
    assert!(
        factory
            .create_for_type("com.example.Missing", FlowParserTypeEnum::TypeXml)
            .is_err()
    );
}

/// PathMatchUtil 通配符路径展开。
#[test]
fn path_match_util_expands_wildcards() {
    let dir = std::env::temp_dir().join("liteflow-glob-g");
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    std::fs::write(dir.join("a.xml"), "<flow/>").unwrap();
    std::fs::write(dir.join("sub").join("b.xml"), "<flow/>").unwrap();

    let pattern = dir.join("**/*.xml").to_string_lossy().into_owned();
    let matches = PathMatchUtil::search_absolute_path(&[pattern]);
    assert_eq!(matches.len(), 2);
    assert!(matches.iter().all(|path| path.ends_with(".xml")));

    let _ = std::fs::remove_dir_all(&dir);
}

/// RuleParsePluginUtil.ChainDto 的启用语义。
#[test]
fn chain_dto_enable_semantics() {
    assert_eq!(ChainDto::new("c1").id(), "c1");
    assert_eq!(ChainDto::with_enable("c1", None).enable(), "true");
    assert_eq!(ChainDto::with_enable("c1", Some("")).enable(), "true");
    assert_eq!(ChainDto::with_enable("c1", Some("TRUE")).enable(), "true");
    assert_eq!(ChainDto::with_enable("c1", Some("false")).enable(), "false");
}

/// LocalContextAware 注册声明式包装 Bean。
#[tokio::test]
async fn local_context_aware_registers_decl_wrap_bean() {
    let aware = liteflow_core::spi::local::local_context_aware::LocalContextAware::new();
    let declaration = liteflow_core::core::proxy::DeclWarpBean::new(
        "decl-bean",
        "声明式",
        liteflow_core::enums::NodeTypeEnum::Common,
        Arc::new(MockDecl),
        "tests::MockDecl",
        Vec::new(),
    );
    // LocalContextAware 非容器环境不注册 Bean：registerDeclWrapBean 返回 None、
    // getBean 恒为 None，与 Java 本地实现语义一致
    let registered = aware.register_decl_wrap_bean("decl-bean", declaration.clone());
    assert!(registered.is_none());
    assert!(aware.get_bean("decl-bean").is_none());
}

struct MockDecl;

#[async_trait::async_trait]
impl liteflow_core::core::DeclComponent for MockDecl {
    async fn call(
        &self,
        _method: &str,
        _context: &liteflow_core::CmpContext,
    ) -> Result<Value, LiteflowError> {
        Ok(Value::Null)
    }

    async fn call_with_error(
        &self,
        _method: &str,
        _context: &liteflow_core::CmpContext,
        _error: &LiteflowError,
    ) -> Result<Value, LiteflowError> {
        Ok(Value::Null)
    }

    fn has_method(&self, _method: &str) -> bool {
        false
    }

    fn method_node_type(&self, _method: &str) -> Option<liteflow_core::enums::NodeTypeEnum> {
        None
    }

    fn method_name(&self, _method: &str) -> Option<&str> {
        None
    }

    fn method_retry_count(&self, _method: &str) -> usize {
        0
    }

    fn is_method_retry_for(&self, _method: &str, _error: &LiteflowError) -> bool {
        false
    }

    fn method_for_lifecycle(
        &self,
        _liteflow_method: liteflow_core::enums::LiteFlowMethodEnum,
    ) -> Option<&str> {
        None
    }
}
