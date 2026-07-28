//! Java EL Parser 与 ParserFactory 固有方法语义回归测试。

use std::fs;
use std::sync::Arc;

use liteflow_core::parser::{
    ClassJsonFlowElParser, ClassParserFactory, ClassXmlFlowElParser, ClassYmlFlowElParser,
    LocalJsonFlowElParser, LocalParserFactory, LocalXmlFlowElParser, LocalYmlFlowElParser,
};
use liteflow_core::{FlowBus, FlowParserTypeEnum, cmp};
use serde_json::Value;
use tempfile::tempdir;

fn bus_with_component() -> FlowBus {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    bus
}

fn json_rule(chain_id: &str) -> String {
    format!(r#"{{"flow":{{"chain":[{{"id":"{chain_id}","body":"THEN(a)"}}]}}}}"#)
}

fn xml_rule(chain_id: &str) -> String {
    format!(r#"<flow><chain id="{chain_id}">THEN(a)</chain></flow>"#)
}

fn yml_rule(chain_id: &str) -> String {
    format!("flow:\n  chain:\n    - id: {chain_id}\n      body: THEN(a)\n")
}

/// 验证自定义 JSON 解析器的固有 `parse_main` 调用真实内容提供器和 FlowBus。
#[test]
fn class_json_parse_main_loads_real_chain() {
    let bus = bus_with_component();
    let parser =
        ClassJsonFlowElParser::new(bus.clone(), Arc::new(|| Ok(json_rule("classJsonChain"))));

    assert_eq!(parser.parse_main(&[]).unwrap(), ["classJsonChain"]);
    assert!(bus.contains_chain("classJsonChain"));
}

/// 验证自定义 XML 解析器的固有 `parse_main` 调用真实内容提供器和 FlowBus。
#[test]
fn class_xml_parse_main_loads_real_chain() {
    let bus = bus_with_component();
    let parser = ClassXmlFlowElParser::new(bus.clone(), Arc::new(|| Ok(xml_rule("classXmlChain"))));

    assert_eq!(parser.parse_main(&[]).unwrap(), ["classXmlChain"]);
    assert!(bus.contains_chain("classXmlChain"));
}

/// 验证自定义 YML 解析器的固有 `parse_main` 调用真实内容提供器和 FlowBus。
#[test]
fn class_yml_parse_main_loads_real_chain() {
    let bus = bus_with_component();
    let parser = ClassYmlFlowElParser::new(bus.clone(), Arc::new(|| Ok(yml_rule("classYmlChain"))));

    assert_eq!(parser.parse_main(&[]).unwrap(), ["classYmlChain"]);
    assert!(bus.contains_chain("classYmlChain"));
}

/// 验证本地 JSON 解析器通过 PathContentParser 读取真实文件。
#[test]
fn local_json_parse_main_reads_real_file() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("rules.json");
    fs::write(&path, json_rule("localJsonChain")).unwrap();
    let bus = bus_with_component();
    let parser = LocalJsonFlowElParser::new(bus.clone());

    assert_eq!(
        parser
            .parse_main(&[path.to_string_lossy().into_owned()])
            .unwrap(),
        ["localJsonChain"]
    );
    assert!(bus.contains_chain("localJsonChain"));
}

/// 验证本地 JSON 解析器通过运行时 classpath 资源装载真实 Chain。
#[test]
fn local_json_parse_main_reads_classpath_resource() {
    let bus = bus_with_component();
    let parser = LocalJsonFlowElParser::new(bus.clone());

    assert_eq!(
        parser
            .parse_main(&["classpath:path_content_parser/rule.json".to_string()])
            .unwrap(),
        ["classpathChain"]
    );
    assert!(bus.contains_chain("classpathChain"));
}

/// 验证本地 XML 解析器通过 PathContentParser 读取真实文件。
#[test]
fn local_xml_parse_main_reads_real_file() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("rules.xml");
    fs::write(&path, xml_rule("localXmlChain")).unwrap();
    let bus = bus_with_component();
    let parser = LocalXmlFlowElParser::new(bus.clone());

    assert_eq!(
        parser
            .parse_main(&[path.to_string_lossy().into_owned()])
            .unwrap(),
        ["localXmlChain"]
    );
    assert!(bus.contains_chain("localXmlChain"));
}

/// 验证本地 YML 解析器通过 PathContentParser 读取真实文件。
#[test]
fn local_yml_parse_main_reads_real_file() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("rules.yml");
    fs::write(&path, yml_rule("localYmlChain")).unwrap();
    let bus = bus_with_component();
    let parser = LocalYmlFlowElParser::new(bus.clone());

    assert_eq!(
        parser
            .parse_main(&[path.to_string_lossy().into_owned()])
            .unwrap(),
        ["localYmlChain"]
    );
    assert!(bus.contains_chain("localYmlChain"));
}

/// 验证 ClassParserFactory 三个固有创建入口共享注册表并校验格式。
#[test]
fn class_factory_creates_registered_real_parsers() {
    let bus = bus_with_component();
    let factory = ClassParserFactory::new(bus.clone());
    factory.register(
        "tests.JsonParser",
        FlowParserTypeEnum::TypeElJson,
        Arc::new(|| Ok(json_rule("factoryJsonChain"))),
    );
    factory.register(
        "tests.XmlParser",
        FlowParserTypeEnum::TypeElXml,
        Arc::new(|| Ok(xml_rule("factoryXmlChain"))),
    );
    factory.register(
        "tests.YmlParser",
        FlowParserTypeEnum::TypeElYml,
        Arc::new(|| Ok(yml_rule("factoryYmlChain"))),
    );

    assert_eq!(
        factory
            .create_json_el_parser("tests.JsonParser")
            .unwrap()
            .parse_main(&[])
            .unwrap(),
        ["factoryJsonChain"]
    );
    assert_eq!(
        factory
            .create_xml_el_parser("tests.XmlParser")
            .unwrap()
            .parse_main(&[])
            .unwrap(),
        ["factoryXmlChain"]
    );
    assert_eq!(
        factory
            .create_yml_el_parser("tests.YmlParser")
            .unwrap()
            .parse_main(&[])
            .unwrap(),
        ["factoryYmlChain"]
    );
    assert!(factory.create_xml_el_parser("tests.JsonParser").is_err());
}

/// 验证 LocalParserFactory 三个固有创建入口返回可读取真实文件的解析器。
#[test]
fn local_factory_creates_real_file_parsers() {
    let directory = tempdir().unwrap();
    let json_path = directory.path().join("factory.json");
    let xml_path = directory.path().join("factory.xml");
    let yml_path = directory.path().join("factory.yml");
    fs::write(&json_path, json_rule("localFactoryJson")).unwrap();
    fs::write(&xml_path, xml_rule("localFactoryXml")).unwrap();
    fs::write(&yml_path, yml_rule("localFactoryYml")).unwrap();

    let factory = LocalParserFactory::new(bus_with_component());
    for (parser, path, expected) in [
        (
            factory
                .create_json_el_parser(json_path.to_string_lossy().as_ref())
                .unwrap(),
            json_path,
            "localFactoryJson",
        ),
        (
            factory
                .create_xml_el_parser(xml_path.to_string_lossy().as_ref())
                .unwrap(),
            xml_path,
            "localFactoryXml",
        ),
        (
            factory
                .create_yml_el_parser(yml_path.to_string_lossy().as_ref())
                .unwrap(),
            yml_path,
            "localFactoryYml",
        ),
    ] {
        assert_eq!(
            parser
                .parse_main(&[path.to_string_lossy().into_owned()])
                .unwrap(),
            [expected]
        );
    }
}
